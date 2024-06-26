mod ui;

use nalgebra as na;
use pollster::FutureExt;
use std::sync::Arc;

fn main() {
	match try_main() {
		Ok(()) => {},
		Err(err) => println!("Error: {}", err),
	}
}

fn try_main() -> Result<(), Error> {
	let event_loop = winit::event_loop::EventLoop::new()?;

	let mut app = App::Starting;
	event_loop.run(|event, event_loop| match event {
		winit::event::Event::Resumed => app.resumed(event_loop),
		winit::event::Event::Suspended => app.suspended(event_loop),
		winit::event::Event::WindowEvent { window_id, event } => app.window_event(event_loop, window_id, event),
		_ => {},
	})?;
	if let App::Error(err) = app {
		return Err(err);
	}
	Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error(transparent)]
	EventLoop(#[from] winit::error::EventLoopError),
}

enum App {
	Starting,
	Running(Program),
	Error(Error),
}

type EventLoop = winit::event_loop::EventLoopWindowTarget<()>;
// wait for egui winit 0.30
// impl winit::application::ApplicationHandler for App {
impl App {
	fn resumed(&mut self, event_loop: &EventLoop) {
		self.try_do(event_loop, |_program| {
			println!("resumed");
			Ok(())
		})
	}

	fn suspended(&mut self, event_loop: &EventLoop) {
		self.try_do(event_loop, |_program| {
			println!("suspended");
			Ok(())
		})
	}

	fn window_event(
		&mut self,
		event_loop: &EventLoop,
		_window_id: winit::window::WindowId,
		event: winit::event::WindowEvent,
	) {
		self.try_do(event_loop, |program| {
			println!("window_event {:?}", event);

			let response = program.egui_winit.on_window_event(&program.window, &event);
			if response.repaint {
				program.window.request_redraw();
			}
			if response.consumed {
				return Ok(());
			}

			match event {
				winit::event::WindowEvent::CloseRequested => {
					event_loop.exit();
				},
				winit::event::WindowEvent::MouseInput { .. } => {
					program.world.injector.push(Task::Load);
				},
				winit::event::WindowEvent::RedrawRequested => {
					program.render();
				},
				winit::event::WindowEvent::Resized(_size) => {
					program.window.resized(&program.world.state);
				},
				_ => {},
			}
			Ok(())
		})
	}
}

impl App {
	pub fn try_do(&mut self, event_loop: &EventLoop, action: impl Fn(&mut Program) -> Result<(), Error>) {
		match self {
			Self::Starting => {
				let mut program = match Program::new(event_loop) {
					Ok(program) => program,
					Err(err) => {
						*self = Self::Error(err);
						event_loop.exit();
						return;
					},
				};
				*self = match action(&mut program) {
					Ok(()) => Self::Running(program),
					Err(err) => {
						event_loop.exit();
						Self::Error(err)
					},
				};
			},
			Self::Running(program) => match action(program) {
				Ok(()) => {},
				Err(err) => {
					event_loop.exit();
					*self = Self::Error(err)
				},
			},
			Self::Error(_) => {},
		}
	}
}

struct Program {
	world: Arc<World>,
	window: render::Window,

	egui: egui::Context,
	egui_winit: egui_winit::State,
	egui_wgpu: egui_wgpu::Renderer,

	display_settings: DisplaySettings,
}

pub struct DisplaySettings {
	background: na::Point3<f32>,
}

struct World {
	state: render::State,
	injector: crossbeam::deque::Injector<Task>,
}

impl Program {
	pub fn new(event_loop: &EventLoop) -> Result<Self, Error> {
		let (state, window) = render::State::new("Treee", event_loop).block_on()?;

		let injector = crossbeam::deque::Injector::<Task>::new();

		let world = World { state, injector };
		let world = Arc::new(world);

		for _ in 0..num_cpus::get() {
			let world = std::sync::Arc::downgrade(&world);
			std::thread::spawn(move || {
				while let Some(world) = world.upgrade() {
					let task = match world.injector.steal() {
						crossbeam::deque::Steal::Success(task) => task,
						crossbeam::deque::Steal::Empty => {
							std::thread::sleep(std::time::Duration::from_millis(100));
							continue;
						},
						crossbeam::deque::Steal::Retry => continue,
					};
					task.run(&world);
				}
			});
		}

		let egui = egui::Context::default();
		let egui_winit = egui_winit::State::new(egui.clone(), egui.viewport_id(), window.inner(), None, None);
		let egui_wgpu = egui_wgpu::Renderer::new(world.state.device(), world.state.surface_format(), None, 1);

		Ok(Self {
			world,
			window,
			egui,
			egui_winit,
			egui_wgpu,
			display_settings: DisplaySettings { background: na::point![0.1, 0.2, 0.3] },
		})
	}

	pub fn render(&mut self) {
		let raw_input = self.egui_winit.take_egui_input(&self.window);
		let full_output = self.egui.run(raw_input, |ctx| {
			egui::SidePanel::left("left")
				.resizable(false)
				.default_width(275.0)
				.show(ctx, |ui| ui::ui(ui, &mut self.display_settings));
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

			let render_pass = context.render_pass(self.display_settings.background);
			drop(render_pass);

			let mut render_pass = context.post_process_pass();
			self.egui_wgpu.render(&mut render_pass, &paint_jobs, screen);
			drop(render_pass);
		});
	}
}

#[derive(Debug)]
enum Task {
	Load,
	Insert,
}

impl Task {
	pub fn run(self, world: &World) {
		match self {
			Self::Load => {
				for _ in 0..20 {
					world.injector.push(Self::Insert);
				}
			},
			Self::Insert => {
				println!("insert");
			},
		}
	}
}
