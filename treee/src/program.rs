use crate::camera::Camera;
use crate::empty::{Empty, EmptyResponse};
use crate::interactive::{Interactive, InteractiveResponse};
use crate::loading::{Loading, LoadingResponse};
use crate::segmenting::{Segmenting, SegmentingResponse, DEFAULT_MAX_DISTANCE};
use crate::{Error, EventLoop};
use nalgebra as na;
use pollster::FutureExt;
use std::sync::Arc;

pub enum Event {
	Done,
	Lookup(render::Lookup),
}

pub struct Program {
	pub world: World,
	pub receiver: crossbeam::channel::Receiver<Event>,

	pub state: Arc<render::State>,
	pub window: render::Window,
	pub keyboard: input::Keyboard,
	pub mouse: input::Mouse,
	time: Time,
	paused: bool,

	pub egui: egui::Context,
	pub egui_winit: egui_winit::State,
	pub egui_wgpu: egui_wgpu::Renderer,

	pub eye_dome: render::EyeDome,
	pub point_cloud_state: render::PointCloudState,

	pub display_settings: DisplaySettings,
}

pub struct DisplaySettings {
	pub background: na::Point3<f32>,
	pub point_cloud_environment: render::PointCloudEnvironment,
	pub lookup: render::Lookup,
	pub camera: Camera,
}

pub enum World {
	Empty(Empty),
	Loading(Arc<Loading>),
	Segmenting(Segmenting),
	Interactive(Arc<Interactive>),
}

impl Program {
	pub fn new(event_loop: &EventLoop) -> Result<Self, Error> {
		let (state, window) = render::State::new("Treee", event_loop).block_on()?;

		let point_cloud_environment = render::PointCloudEnvironment::new(&state, 0, u32::MAX, 0.1);
		let point_cloud_state = render::PointCloudState::new(&state);
		let lookup = render::Lookup::new_png(
			&state,
			include_bytes!("../../viewer/assets/grad_warm.png"),
			u32::MAX,
		);
		let camera = Camera::new(&state, window.get_aspect());
		let eye_dome = render::EyeDome::new(&state, window.config(), window.depth_texture(), 0.7);

		let egui = egui::Context::default();
		let egui_winit = egui_winit::State::new(egui.clone(), egui.viewport_id(), window.inner(), None, None);
		let egui_wgpu = egui_wgpu::Renderer::new(state.device(), state.surface_format(), None, 1);

		let (empty, receiver) = Empty::new();
		Ok(Self {
			world: World::Empty(empty),
			receiver,

			state: Arc::new(state),
			window,
			egui,
			egui_winit,
			egui_wgpu,

			paused: false,
			keyboard: input::Keyboard::new(),
			mouse: input::Mouse::new(),
			time: Time::new(),

			eye_dome,
			point_cloud_state,

			display_settings: DisplaySettings {
				background: na::point![0.1, 0.2, 0.3],
				point_cloud_environment,
				lookup,
				camera,
			},
		})
	}

	pub fn render(&mut self) {
		if self.paused {
			return;
		}
		let raw_input = self.egui_winit.take_egui_input(&self.window);
		let full_output = self.egui.run(raw_input, |ctx| {
			egui::TopBottomPanel::top("panel")
				.resizable(false)
				.show(ctx, |ui| {
					ui.horizontal(|ui| match &mut self.world {
						World::Empty(empty) => match empty.ui(ui) {
							EmptyResponse::None => {},
							EmptyResponse::Load(path) => {
								let (loading, receiver) = Loading::new(path, self.state.clone());
								self.world = World::Loading(loading);
								self.receiver = receiver;
							},
						},
						World::Loading(loading) => match loading.ui(ui, &mut self.display_settings) {
							LoadingResponse::None => {},
							LoadingResponse::Close => {
								let (empty, receiver) = Empty::new();
								self.world = World::Empty(empty);
								self.receiver = receiver;
							},
						},
						World::Segmenting(segmenting) => match segmenting.ui(ui) {
							SegmentingResponse::None => {},
							SegmentingResponse::Done(segments) => {
								let (interative, receiver) = Interactive::new();
								self.world = World::Interactive(interative);
								self.receiver = receiver;
							},
						},
						World::Interactive(interactive) => match interactive.ui(ui) {
							InteractiveResponse::None => {},
						},
					});
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
			pixels_per_point: 1.0,
		};
		for (id, delta) in full_output.textures_delta.set {
			self.egui_wgpu
				.update_texture(&self.state.device, &self.state.queue, id, &delta);
		}
		for id in full_output.textures_delta.free {
			self.egui_wgpu.free_texture(&id);
		}

		match &self.world {
			World::Empty(_empty) => {
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
					let _ = context.render_pass(self.display_settings.background);

					let mut render_pass = context.post_process_pass();
					self.egui_wgpu.render(&mut render_pass, &paint_jobs, screen);
					drop(render_pass);
				});
			},
			World::Loading(loading) => {
				let point_clouds = loading.point_clouds.lock().unwrap();
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
					loading
						.octree
						.render(point_cloud_pass, &point_clouds, &loading.property);
					drop(render_pass);

					let mut render_pass = context.post_process_pass();
					self.eye_dome.render(&mut render_pass);
					self.egui_wgpu.render(&mut render_pass, &paint_jobs, screen);
					drop(render_pass);
				});
			},
			World::Segmenting(segmenting) => {
				let point_clouds = segmenting.shared.point_clouds.lock().unwrap();
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
					for (point_cloud, property) in point_clouds.iter() {
						point_cloud.render(point_cloud_pass, property);
					}
					drop(render_pass);

					let mut render_pass = context.post_process_pass();
					self.eye_dome.render(&mut render_pass);
					self.egui_wgpu.render(&mut render_pass, &paint_jobs, screen);
					drop(render_pass);
				});
			},
			World::Interactive(interactive) => {
				// let point_clouds = interactive.point_clouds.lock().unwrap();
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
					// let point_cloud_pass = self.point_cloud_state.render(
					// 	&mut render_pass,
					// 	self.display_settings.camera.gpu(),
					// 	&self.display_settings.lookup,
					// 	&self.display_settings.point_cloud_environment,
					// );
					// for (point_cloud, property) in point_clouds.iter() {
					// 	point_cloud.render(point_cloud_pass, property);
					// }
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
		if self.keyboard.pressed(input::KeyCode::KeyD) || self.keyboard.pressed(input::KeyCode::ArrowRight) {
			direction.x += 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyS) || self.keyboard.pressed(input::KeyCode::ArrowDown) {
			direction.y += 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyA) || self.keyboard.pressed(input::KeyCode::ArrowLeft) {
			direction.x -= 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyW) || self.keyboard.pressed(input::KeyCode::ArrowUp) {
			direction.y -= 1.0;
		}
		let l = direction.norm();
		if l > 0.0 {
			direction *= 10.0 * delta / l;
			self.display_settings
				.camera
				.movement(direction, &self.state);
		}

		while let Ok(event) = self.receiver.try_recv() {
			match event {
				Event::Lookup(lookup) => self.display_settings.lookup = lookup,
				Event::Done => {
					match std::mem::replace(&mut self.world, World::Empty(Empty::new().0)) {
						World::Loading(loading) => {
							let loading = Arc::try_unwrap(loading).unwrap();
							let (segmenting, receiver) =
								Segmenting::new(loading.octree, self.state.clone(), DEFAULT_MAX_DISTANCE);
							self.world = World::Segmenting(segmenting);
							self.receiver = receiver;
						},
						world @ _ => self.world = world,
					};
				},
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
}

struct Time {
	last: std::time::Instant,
}

impl Time {
	pub fn new() -> Self {
		Self { last: std::time::Instant::now() }
	}

	pub fn elapsed(&mut self) -> std::time::Duration {
		let delta = self.last.elapsed();
		self.last = std::time::Instant::now();
		delta
	}
}
