use crate::calculations::{Calculations, DisplayModus};
use crate::camera::Camera;
use crate::empty::Empty;
use crate::interactive::{self, Interactive, DELETED_INDEX};
use crate::loading::Loading;
use crate::segmenting::{Segmenting, DEFAULT_MAX_DISTANCE};
use crate::{environment, Error, EventLoop};
use nalgebra as na;
use render::PointCloudPass;
use std::collections::HashMap;
use std::ops::Not;
use std::sync::Arc;

pub enum Event {
	Done,
	Lookup {
		bytes: &'static [u8],
		max: u32,
	},
	ClearPointClouds,
	PointCloud {
		idx: Option<u32>,
		data: Vec<na::Point3<f32>>,
		segment: Vec<u32>,
		property: Option<Vec<u32>>,
	},
	RemovePointCloud(u32),
	PointCloudProperty {
		idx: u32,
		data: Vec<u32>,
	},
	Load(environment::Source),
	Segmented {
		segments: HashMap<u32, Vec<na::Point3<f32>>>,
		world_offset: na::Point3<f64>,
	},
}

pub struct Program {
	pub world: World,
	pub receiver: crossbeam::channel::Receiver<Event>,

	pub state: render::State,
	pub window: render::Window,
	pub keyboard: input::Keyboard,

	mouse: input::Mouse,
	mouse_start: Option<na::Point2<f32>>,
	time: Time,
	paused: bool,

	pub egui: egui::Context,
	pub egui_winit: egui_winit::State,
	pub egui_wgpu: egui_wgpu::Renderer,

	pub eye_dome: render::EyeDome,
	pub point_cloud_state: render::PointCloudState,
	pub lines_state: render::LinesState,

	pub display_settings: DisplaySettings,

	chunks: HashMap<u32, Chunk>,
}

struct Chunk {
	point_cloud: render::PointCloud,
	segment: render::PointCloudProperty,
	property: Option<render::PointCloudProperty>,
}

impl Chunk {
	pub fn render<'a>(&'a self, display: DisplayModus, point_cloud_pass: &mut PointCloudPass<'a>) {
		match display {
			DisplayModus::Segment => self.point_cloud.render(point_cloud_pass, &self.segment),
			DisplayModus::Property => self
				.point_cloud
				.render(point_cloud_pass, &self.property.as_ref().unwrap()),
		}
	}
}

pub struct DisplaySettings {
	pub background: na::Point3<f32>,
	pub point_cloud_environment: render::PointCloudEnvironment,
	pub lookup: render::Lookup,
	pub camera: Camera,
}

pub enum World {
	Empty(Empty),
	Loading(Loading),
	Segmenting(Segmenting),
	Calculations(Calculations),
	Interactive(Interactive),
}

impl Program {
	pub async fn new(event_loop: &EventLoop) -> Result<Self, Error> {
		let window = winit::window::WindowBuilder::new()
			.with_title("Treee")
			.with_min_inner_size(winit::dpi::LogicalSize { width: 10, height: 10 })
			.build(event_loop)
			.unwrap();

		#[cfg(target_arch = "wasm32")]
		{
			use winit::platform::web::WindowExtWebSys;
			web_sys::window()
				.and_then(|win| win.document())
				.and_then(|doc| {
					let dst = doc.get_element_by_id("wasm-example")?;
					let canvas = web_sys::Element::from(window.canvas()?);
					dst.append_child(&canvas).ok()?;
					Some(())
				})
				.expect("Couldn't append canvas to document body.");
		}
		let window = Arc::new(window);

		let (state, window) = render::State::new(window).await?;

		let point_cloud_environment = render::PointCloudEnvironment::new(&state, 0, u32::MAX, 0.1);
		let point_cloud_state = render::PointCloudState::new(&state);
		let lines_state = render::LinesState::new(&state);
		let lookup =
			render::Lookup::new_png(&state, include_bytes!("../assets/grad_warm.png"), u32::MAX);
		let camera = Camera::new(&state, window.get_aspect());
		let eye_dome = render::EyeDome::new(&state, window.config(), window.depth_texture(), 0.7);

		let egui = egui::Context::default();
		let egui_winit = egui_winit::State::new(
			egui.clone(),
			egui.viewport_id(),
			window.inner(),
			Some(window.scale_factor() as f32),
			None,
		);
		let egui_wgpu = egui_wgpu::Renderer::new(state.device(), state.surface_format(), None, 1);

		let (empty, receiver) = Empty::new();
		Ok(Self {
			world: World::Empty(empty),
			receiver,

			state,
			window,
			egui,
			egui_winit,
			egui_wgpu,

			paused: false,
			keyboard: input::Keyboard::new(),
			mouse: input::Mouse::new(),
			mouse_start: None,
			time: Time::new(),

			eye_dome,
			point_cloud_state,
			lines_state,

			display_settings: DisplaySettings {
				background: na::point![0.3, 0.5, 0.7],
				point_cloud_environment,
				lookup,
				camera,
			},

			chunks: HashMap::new(),
		})
	}

	pub fn render(&mut self) {
		if self.paused {
			return;
		}
		let raw_input = self.egui_winit.take_egui_input(&self.window);
		let full_output = self.egui.run(raw_input, |ctx| {
			egui::SidePanel::left("panel")
				.resizable(false)
				.min_width(200.0)
				.show(ctx, |ui| {
					if matches!(self.world, World::Empty(_)).not() {
						if ui
							.add_sized([ui.available_width(), 0.0], egui::Button::new("Close"))
							.clicked()
						{
							let (empty, reciever) = Empty::new();
							self.world = World::Empty(empty);
							self.receiver = reciever;
						}
					}
					match &mut self.world {
						World::Empty(empty) => empty.ui(ui),
						World::Loading(loading) => loading.ui(ui),
						World::Segmenting(segmenting) => segmenting.ui(ui),
						World::Calculations(calculations) => calculations.ui(ui),
						World::Interactive(interactive) => interactive.ui(ui, &self.state),
					}
				});
		});
		self.egui_winit
			.handle_platform_output(&self.window, full_output.platform_output);

		let paint_jobs = self
			.egui
			.tessellate(full_output.shapes, full_output.pixels_per_point);

		let config = self.window.config();
		let screen = &egui_wgpu::ScreenDescriptor {
			size_in_pixels: [config.width, config.height],
			pixels_per_point: self.egui.pixels_per_point(),
		};
		for (id, delta) in full_output.textures_delta.set {
			self.egui_wgpu
				.update_texture(&self.state.device, &self.state.queue, id, &delta);
		}
		for id in full_output.textures_delta.free {
			self.egui_wgpu.free_texture(&id);
		}

		match &self.world {
			World::Empty(_) | World::Loading(_) | World::Segmenting(_) => {
				self.window.render(&self.state, |context| {
					let command_encoder = context.encoder();
					let commands = self.egui_wgpu.update_buffers(
						&self.state.device,
						&self.state.queue,
						command_encoder,
						&paint_jobs,
						screen,
					);
					self.state.queue.submit(commands);
					let mut render_pass = context.render_pass(self.display_settings.background);
					let point_cloud_pass = self.point_cloud_state.render(
						&mut render_pass,
						self.display_settings.camera.gpu(),
						&self.display_settings.lookup,
						&self.display_settings.point_cloud_environment,
					);

					for (_, chunk) in self.chunks.iter() {
						chunk.render(DisplayModus::Segment, point_cloud_pass);
					}
					drop(render_pass);

					let mut render_pass = context.post_process_pass();
					self.eye_dome.render(&mut render_pass);
					self.egui_wgpu.render(&mut render_pass, &paint_jobs, screen);
					drop(render_pass);
				});
			},
			&World::Calculations(Calculations { display, .. }) => {
				self.window.render(&self.state, |context| {
					let command_encoder = context.encoder();
					let commands = self.egui_wgpu.update_buffers(
						&self.state.device,
						&self.state.queue,
						command_encoder,
						&paint_jobs,
						screen,
					);
					self.state.queue.submit(commands);

					let mut render_pass = context.render_pass(self.display_settings.background);
					let point_cloud_pass = self.point_cloud_state.render(
						&mut render_pass,
						self.display_settings.camera.gpu(),
						&self.display_settings.lookup,
						&self.display_settings.point_cloud_environment,
					);

					for (_, chunk) in self.chunks.iter() {
						chunk.render(display, point_cloud_pass);
					}
					drop(render_pass);

					let mut render_pass = context.post_process_pass();
					self.eye_dome.render(&mut render_pass);
					self.egui_wgpu.render(&mut render_pass, &paint_jobs, screen);
					drop(render_pass);
				});
			},
			World::Interactive(interactive) => {
				self.window.render(&self.state, |context| {
					let command_encoder = context.encoder();
					let commands = self.egui_wgpu.update_buffers(
						&self.state.device,
						&self.state.queue,
						command_encoder,
						&paint_jobs,
						screen,
					);
					self.state.queue.submit(commands);

					let mut render_pass = context.render_pass(self.display_settings.background);
					let point_cloud_pass = self.point_cloud_state.render(
						&mut render_pass,
						self.display_settings.camera.gpu(),
						&self.display_settings.lookup,
						&self.display_settings.point_cloud_environment,
					);
					if interactive.show_deleted {
						if let Some(chunk) = self.chunks.get(&DELETED_INDEX) {
							point_cloud_pass.lookup(&interactive.white_lookup);
							chunk.point_cloud.render(point_cloud_pass, &chunk.segment);
							point_cloud_pass.lookup(&self.display_settings.lookup);
						}
					}
					if let interactive::Modus::View(idx, _) = interactive.modus {
						let chunk = self.chunks.get(&idx).unwrap();
						chunk.render(interactive.display, point_cloud_pass);
					} else {
						for (_, chunk) in
							self.chunks.iter().filter(|&(&idx, _)| idx != DELETED_INDEX)
						{
							chunk.render(interactive.display, point_cloud_pass);
						}
					}
					if let interactive::Modus::View(idx, mesh) = &interactive.modus {
						if let Some(mesh) = mesh {
							let lines_pass = self
								.lines_state
								.render(&mut render_pass, self.display_settings.camera.gpu());
							let seg = self.chunks.get(idx).unwrap();
							mesh.lines.render(&seg.point_cloud, lines_pass);
						};
					}

					drop(render_pass);

					let mut render_pass = context.post_process_pass();
					self.eye_dome.render(&mut render_pass);
					self.egui_wgpu.render(&mut render_pass, &paint_jobs, screen);
					drop(render_pass);
				});
			},
		}
	}

	pub fn update(&mut self) -> Result<(), Error> {
		let delta = self.time.elapsed().as_secs_f32();
		let mut direction = na::vector![0.0, 0.0];
		if self.keyboard.pressed(input::KeyCode::KeyD)
			|| self.keyboard.pressed(input::KeyCode::ArrowRight)
		{
			direction.x += 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyS)
			|| self.keyboard.pressed(input::KeyCode::ArrowDown)
		{
			direction.y += 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyA)
			|| self.keyboard.pressed(input::KeyCode::ArrowLeft)
		{
			direction.x -= 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyW)
			|| self.keyboard.pressed(input::KeyCode::ArrowUp)
		{
			direction.y -= 1.0;
		}
		let l = direction.norm();
		if l > 0.0 {
			direction *= delta / l;
			self.display_settings
				.camera
				.movement(direction, &self.state);
		}
		if self.keyboard.pressed(input::KeyCode::KeyQ) {
			self.display_settings
				.camera
				.vertical_movement(delta * -10.0, &self.state);
		}
		if self.keyboard.pressed(input::KeyCode::KeyE) {
			self.display_settings
				.camera
				.vertical_movement(delta * 10.0, &self.state);
		}

		let mut work = self.receiver.len();
		while let Ok(event) = self.receiver.try_recv() {
			match event {
				Event::Load(source) => match source.extension() {
					"laz" | "las" => match &mut self.world {
						World::Loading(loading) => loading.add(source),
						_ => {
							let (loading, receiver) = Loading::new(source);
							self.world = World::Loading(loading);
							self.receiver = receiver;
						},
					},

					"ipc" => {
						let (interactive, receiver) = Interactive::load(source, &self.state);
						self.world = World::Interactive(interactive);
						self.receiver = receiver;
					},
					_ => panic!("invalid file format"),
				},
				Event::Lookup { bytes, max } => {
					self.display_settings.lookup = render::Lookup::new_png(&self.state, &bytes, max)
				},
				Event::Done => {
					match std::mem::replace(&mut self.world, World::Empty(Empty::new().0)) {
						World::Loading(loading) => {
							let (segmenting, receiver) =
								Segmenting::new(loading, DEFAULT_MAX_DISTANCE);
							self.world = World::Segmenting(segmenting);
							self.receiver = receiver;
						},
						World::Calculations(calculations) => {
							let shared = Arc::try_unwrap(calculations.shared).unwrap();
							let (interactive, receiver) = Interactive::new(
								shared.segments.into_inner().unwrap(),
								&self.state,
								calculations.display,
								calculations.world_offset,
							);
							self.world = World::Interactive(interactive);
							self.receiver = receiver;
						},
						world => self.world = world,
					};
				},
				Event::Segmented { segments, world_offset } => {
					let (calculations, receiver) = Calculations::new(segments, world_offset);
					self.world = World::Calculations(calculations);
					self.receiver = receiver;
				},

				Event::ClearPointClouds => {
					self.chunks.clear();
				},

				Event::PointCloud { idx, data, segment, property } => {
					let idx = idx.unwrap_or_else(|| {
						let mut idx = rand::random();
						while self.chunks.contains_key(&idx) {
							idx = rand::random();
						}
						idx
					});
					self.chunks.insert(
						idx,
						Chunk {
							point_cloud: render::PointCloud::new(&self.state, &data),
							segment: render::PointCloudProperty::new(&self.state, &segment),
							property: property.map(|property| {
								render::PointCloudProperty::new(&self.state, &property)
							}),
						},
					);
				},
				Event::PointCloudProperty { idx, data } => {
					if let Some(chunk) = self.chunks.get_mut(&idx) {
						chunk.property = Some(render::PointCloudProperty::new(&self.state, &data));
					}
				},
				Event::RemovePointCloud(idx) => {
					self.chunks.remove(&idx);
				},
			}
			work -= 1;
			if work == 0 {
				break;
			}
		}

		Ok(())
	}

	pub fn resized(&mut self) {
		if self.window.inner_size().width == 0 || self.window.inner_size().height == 0 {
			self.paused = true;
			return;
		}
		self.paused = false;
		self.window.resized(&self.state);
		self.display_settings
			.camera
			.update_aspect(self.window.get_aspect(), &self.state);
		self.eye_dome
			.update_depth(&self.state, self.window.depth_texture());
	}

	pub fn mouse_click(&mut self, button: input::MouseButton, state: input::State) {
		self.mouse.update(button, state);
		self.window.request_redraw();

		match (button, state) {
			(input::MouseButton::Left, input::State::Pressed) => {
				self.mouse_start = Some(self.mouse.position());
			},
			(input::MouseButton::Left, input::State::Released) => {
				let Some(start) = self.mouse_start else {
					return;
				};
				let dist = (start - self.mouse.position()).norm();
				if dist >= 2.0 {
					return;
				}
				let World::Interactive(interactive) = &mut self.world else {
					return;
				};
				interactive.click(
					self.display_settings.camera.position(),
					self.display_settings
						.camera
						.ray_direction(self.mouse.position(), self.window.get_size()),
				);
			},
			(input::MouseButton::Right, input::State::Pressed) => {
				let World::Interactive(interactive) = &mut self.world else {
					return;
				};
				interactive.drag(
					self.display_settings.camera.position(),
					self.display_settings
						.camera
						.ray_direction(self.mouse.position(), self.window.get_size()),
					&self.state,
				);
			},
			_ => {},
		}
	}

	pub fn key(&mut self, key: input::KeyCode, state: input::State) {
		self.keyboard.update(key, state);
	}

	pub fn mouse_move(&mut self, position: na::Point2<f32>) {
		self.window.request_redraw();
		let delta = self.mouse.delta(position);
		if self.mouse.pressed(input::MouseButton::Left) {
			self.display_settings.camera.rotate(delta, &self.state);
		} else if self.mouse.pressed(input::MouseButton::Right) {
			let World::Interactive(interactive) = &mut self.world else {
				return;
			};
			interactive.drag(
				self.display_settings.camera.position(),
				self.display_settings
					.camera
					.ray_direction(self.mouse.position(), self.window.get_size()),
				&self.state,
			);
		}
	}
}

struct Time {
	now: web_time::Instant,
}

impl Time {
	pub fn new() -> Self {
		Self { now: web_time::Instant::now() }
	}

	pub fn elapsed(&mut self) -> web_time::Duration {
		let now = web_time::Instant::now();
		let delta = now - self.now;
		self.now = now;
		delta
	}
}
