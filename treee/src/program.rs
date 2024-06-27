use crate::octree::Octree;
use crate::{camera::Camera, task::Task};
use crate::{Error, EventLoop};
use nalgebra as na;
use pollster::FutureExt;
use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

use crate::task::TaskResult;

pub struct Program {
	pub world: Arc<World>,
	pub window: render::Window,
	pub reciever: crossbeam::channel::Receiver<TaskResult>,

	pub keyboard: input::Keyboard,
	pub mouse: input::Mouse,
	time: Time,
	paused: bool,

	pub egui: egui::Context,
	pub egui_winit: egui_winit::State,
	pub egui_wgpu: egui_wgpu::Renderer,
	pub eye_dome: render::EyeDome,

	pub display_settings: DisplaySettings,
}

pub struct DisplaySettings {
	pub background: na::Point3<f32>,
	pub point_cloud_environment: render::PointCloudEnvironment,
	pub lookup: render::Lookup,
	pub camera: Camera,
}

pub struct World {
	pub state: render::State,
	pub task_sender: crossbeam::channel::Sender<Task>,
	pub sender: crossbeam::channel::Sender<TaskResult>,
	pub octree: RwLock<Octree>,

	pub point_cloud_state: render::PointCloudState,
	pub point_clouds: std::sync::Mutex<HashMap<usize, (render::PointCloud, render::PointCloudProperty)>>,
	pub fallback_property: render::PointCloudProperty,
}

impl Program {
	pub fn new(event_loop: &EventLoop) -> Result<Self, Error> {
		let (state, window) = render::State::new("Treee", event_loop).block_on()?;

		let (task_sender, task_reciever) = crossbeam::channel::unbounded();
		let (sender, reciever) = crossbeam::channel::unbounded();

		let point_cloud_state = render::PointCloudState::new(&state);
		let point_clouds = std::sync::Mutex::new(HashMap::new());
		let fallback_property = render::PointCloudProperty::new_empty(&state);
		let point_cloud_environment = render::PointCloudEnvironment::new(&state, 0, u32::MAX, 0.1);
		let lookup = render::Lookup::new_png(
			&state,
			include_bytes!("../../viewer/assets/grad_warm.png"),
			u32::MAX,
		);
		let camera = Camera::new(&state, window.get_aspect());
		let eye_dome = render::EyeDome::new(&state, window.config(), window.depth_texture(), 0.7);

		let world = World {
			state,
			task_sender,
			sender,
			octree: RwLock::new(Octree::new(na::point![0.0, 0.0, 0.0], 0.0, 0)),

			point_cloud_state,
			point_clouds,
			fallback_property,
		};
		let world = Arc::new(world);

		for _ in 0..num_cpus::get() {
			let world = world.clone();
			let reciever = task_reciever.clone();
			std::thread::spawn(move || {
				for task in reciever {
					if let Err(error) = task.run(&world) {
						world.sender.send(TaskResult::Error(error)).unwrap();
					}
				}
				println!("Worker exit");
			});
		}

		let egui = egui::Context::default();
		let egui_winit = egui_winit::State::new(egui.clone(), egui.viewport_id(), window.inner(), None, None);
		let egui_wgpu = egui_wgpu::Renderer::new(world.state.device(), world.state.surface_format(), None, 1);

		Ok(Self {
			world,
			window,
			reciever,
			egui,
			egui_winit,
			egui_wgpu,

			paused: false,
			keyboard: input::Keyboard::new(),
			mouse: input::Mouse::new(),
			time: Time::new(),

			eye_dome,

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
					ui.horizontal(|ui| crate::ui::ui(ui, &mut self.display_settings, &self.world))
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
			self.egui_wgpu.update_texture(
				&self.world.state.device,
				&self.world.state.queue,
				id,
				&delta,
			);
		}
		for id in full_output.textures_delta.free {
			self.egui_wgpu.free_texture(&id);
		}
		let octree = self.world.octree.read().unwrap();
		let point_clouds = self.world.point_clouds.lock().unwrap();

		self.window.render(&self.world.state, |context| {
			let command_encoder = context.encoder();
			let commands = self.egui_wgpu.update_buffers(
				&self.world.state.device,
				&self.world.state.queue,
				command_encoder,
				&paint_jobs,
				screen,
			);
			self.world.state.queue.submit(commands);

			let mut render_pass = context.render_pass(self.display_settings.background);
			let point_cloud_pass = self.world.point_cloud_state.render(
				&mut render_pass,
				self.display_settings.camera.gpu(),
				&self.display_settings.lookup,
				&self.display_settings.point_cloud_environment,
			);
			octree.render(point_cloud_pass, &point_clouds);
			drop(render_pass);

			let mut render_pass = context.post_process_pass();
			self.eye_dome.render(&mut render_pass);
			self.egui_wgpu.render(&mut render_pass, &paint_jobs, screen);
			drop(render_pass);
		});
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
				.movement(direction, &self.world.state);
		}

		while let Ok(res) = self.reciever.try_recv() {
			match res {
				TaskResult::Error(error) => return Err(error),
				TaskResult::Lookup(lookup) => self.display_settings.lookup = lookup,
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
		self.window.resized(&self.world.state);
		self.display_settings
			.camera
			.update_aspect(self.window.get_aspect(), &self.world.state);
		self.eye_dome
			.update_depth(&self.world.state, self.window.depth_texture());
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
