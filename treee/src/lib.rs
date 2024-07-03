mod calculations;
mod camera;
mod empty;
mod interactive;
mod laz;
mod loading;
mod program;
mod segmenting;

use interactive::InteractiveSave;
use nalgebra as na;
use program::Event;
use program::Program;
use std::io::{BufReader, Read, Seek};
use std::{fs::File, io::BufWriter};

pub async fn try_main() -> Result<(), Error> {
	let event_loop = winit::event_loop::EventLoop::new()?;

	let program = Program::new(&event_loop).await?;
	let mut app = App::Running(program);

	#[cfg(not(target_arch = "wasm32"))]
	{
		event_loop.run(|event, event_loop| match event {
			winit::event::Event::Resumed => app.resumed(event_loop),
			winit::event::Event::Suspended => app.suspended(event_loop),
			winit::event::Event::WindowEvent { window_id, event } => {
				app.window_event(event_loop, window_id, event)
			},
			winit::event::Event::AboutToWait => app.about_to_wait(event_loop),
			_ => {},
		})?;

		if let App::Error(err) = app {
			return Err(err);
		}
	}

	#[cfg(target_arch = "wasm32")]
	{
		use winit::platform::web::EventLoopExtWebSys;
		event_loop.spawn(move |event, event_loop| match event {
			winit::event::Event::Resumed => app.resumed(event_loop),
			winit::event::Event::Suspended => app.suspended(event_loop),
			winit::event::Event::WindowEvent { window_id, event } => {
				app.window_event(event_loop, window_id, event)
			},
			winit::event::Event::AboutToWait => app.about_to_wait(event_loop),
			_ => {},
		});
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
				winit::event::WindowEvent::ScaleFactorChanged { .. } => {
					program.resized();
				},
				winit::event::WindowEvent::KeyboardInput { event, .. } => {
					match event.physical_key {
						winit::keyboard::PhysicalKey::Code(key) => program.key(key, event.state),
						winit::keyboard::PhysicalKey::Unidentified(_) => {},
					}
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
						winit::event::MouseScrollDelta::PixelDelta(pos) => -pos.y as f32 / 100.0,
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
	pub fn try_do(
		&mut self,
		event_loop: &EventLoop,
		action: impl FnOnce(&mut Program) -> Result<(), Error>,
	) {
		match self {
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

#[cfg(not(target_arch = "wasm32"))]
pub mod environment {

	use super::*;

	pub type Source = std::path::PathBuf;

	pub fn get_source(sender: &crossbeam::channel::Sender<Event>) {
		let path = rfd::FileDialog::new()
			.set_title("Load")
			.add_filter("Pointcloud", &["las", "laz", "ipc"])
			.pick_file();
		if let Some(path) = path {
			sender.send(Event::Load(path)).unwrap();
		}
	}

	pub fn save(data: InteractiveSave) {
		rayon::spawn(move || {
			let path = rfd::FileDialog::new()
				.set_title("Save")
				.add_filter("Pointcloud", &["ipc"])
				.save_file();
			if let Some(path) = path {
				let file = File::create(path).unwrap();
				bincode::serialize_into(BufWriter::new(file), &data).unwrap();
			}
		});
	}

	pub fn reader<'a>(source: &'a Source) -> impl Read + Seek + 'a {
		BufReader::new(std::fs::File::open(source).unwrap())
	}

	pub fn extension(source: &Source) -> &str {
		source.extension().unwrap().to_str().unwrap()
	}
}

#[cfg(target_arch = "wasm32")]
pub mod environment {
	use super::*;

	pub type Source = (Vec<u8>, String);

	pub fn get_source(sender: &crossbeam::channel::Sender<Event>) {
		let sender = sender.clone();
		wasm_bindgen_futures::spawn_local(async move {
			let handle = rfd::AsyncFileDialog::new()
				.set_title("Load")
				.add_filter("Pointcloud", &["las", "laz", "ipc"])
				.pick_file()
				.await;
			if let Some(handle) = handle {
				let data = handle.read().await;
				let name = handle.file_name();
				sender.send(Event::Load((data, name))).unwrap();
			}
		});
	}

	pub fn save(data: InteractiveSave) {
		wasm_bindgen_futures::spawn_local(async move {
			let handle = rfd::AsyncFileDialog::new()
				.set_file_name("pointcloud.ipc")
				.set_title("Save")
				.save_file()
				.await;
			if let Some(handle) = handle {
				let mut binary = Vec::new();
				bincode::serialize_into(&mut binary, &data).unwrap();
				handle.write(&binary).await.unwrap();
			}
		});
	}

	pub fn reader<'a>(source: &'a Source) -> impl Read + Seek + 'a {
		std::io::Cursor::new(&source.0)
	}

	pub fn extension(source: &Source) -> &str {
		source.1.split(".").last().unwrap()
	}
}
