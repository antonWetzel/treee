mod camera;
mod empty;
mod interactive;
mod laz;
mod loading;
mod octree;
mod program;
mod segmenting;

use nalgebra as na;
use program::Program;
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;

fn main() {
	match try_main() {
		Ok(()) => {},
		Err(err) => println!("Error: {}", err),
	}
}

fn try_main() -> Result<(), Error> {
	let mut event_loop = winit::event_loop::EventLoop::new()?;

	let mut app = App::Starting;
	event_loop.run_on_demand(|event, event_loop| match event {
		winit::event::Event::Resumed => app.resumed(event_loop),
		winit::event::Event::Suspended => app.suspended(event_loop),
		winit::event::Event::WindowEvent { window_id, event } => app.window_event(event_loop, window_id, event),
		winit::event::Event::AboutToWait => app.about_to_wait(event_loop),
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

	#[error(transparent)]
	IO(#[from] std::io::Error),

	#[error(transparent)]
	LasZip(#[from] ::laz::LasZipError),

	#[error("Corrupt file")]
	CorruptFile,
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

	fn about_to_wait(&mut self, event_loop: &EventLoop) {
		self.try_do(event_loop, |program| {
			program.update()?;
			Ok(())
		})
	}

	fn window_event(
		&mut self,
		event_loop: &EventLoop,
		_window_id: winit::window::WindowId,
		event: winit::event::WindowEvent,
	) {
		self.try_do(event_loop, move |program| {
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
				winit::event::WindowEvent::RedrawRequested => {
					program.render();
				},
				winit::event::WindowEvent::Resized(_size) => {
					program.resized();
				},
				winit::event::WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
					winit::keyboard::PhysicalKey::Code(key) => program.keyboard.update(key, event.state),
					winit::keyboard::PhysicalKey::Unidentified(_) => {},
				},
				winit::event::WindowEvent::MouseInput { state, button, .. } => {
					program.mouse_click(button.into(), state);
				},
				winit::event::WindowEvent::CursorMoved { position, .. } => {
					let position = na::point![position.x as f32, position.y as f32];
					program.mouse_move(position);
				},
				winit::event::WindowEvent::MouseWheel { delta, .. } => {
					let delta = match delta {
						winit::event::MouseScrollDelta::LineDelta(_, y) => -y,
						winit::event::MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
					};
					program
						.display_settings
						.camera
						.scroll(delta, &program.state);
				},
				_ => {},
			}
			Ok(())
		})
	}
}

impl App {
	pub fn try_do(&mut self, event_loop: &EventLoop, action: impl FnOnce(&mut Program) -> Result<(), Error>) {
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
