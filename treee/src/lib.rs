mod calculations;
mod camera;
mod empty;
mod interactive;
mod laz;
mod loading;
mod program;
mod segmenting;

use nalgebra as na;
use program::Event;
use program::Program;
use std::io::{Read, Seek, Write};
use std::sync::Arc;

/// Main loop
pub async fn try_main(error_handler: fn(Error)) {
	let event_loop = match winit::event_loop::EventLoop::with_user_event().build() {
		Ok(v) => v,
		Err(err) => return error_handler(err.into()),
	};

	let proxy = event_loop.create_proxy();

	let app = App {
		state: State::Starting(proxy),
		error_handler,
	};

	#[cfg(not(target_arch = "wasm32"))]
	{
		let mut app = app;
		if let Err(err) = event_loop.run_app(&mut app) {
			return error_handler(err.into());
		};
	}
	#[cfg(target_arch = "wasm32")]
	{
		use winit::platform::web::EventLoopExtWebSys;
		event_loop.spawn_app(app);
	}
}

/// Possible errors for treee
#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Proxy Error")]
	ProxyError,

	#[error(transparent)]
	EventLoop(#[from] winit::error::EventLoopError),

	#[error(transparent)]
	OsError(#[from] winit::error::OsError),

	#[error(transparent)]
	IO(#[from] std::io::Error),

	#[error(transparent)]
	LasZip(#[from] ::laz::LasZipError),

	#[error(transparent)]
	Render(#[from] render::RenderError),

	#[error("Corrupt file")]
	CorruptFile,

	#[error(transparent)]
	Bincode(#[from] bincode::Error),
}

/// App state
struct App {
	state: State,
	error_handler: fn(Error),
}

type Proxy = winit::event_loop::EventLoopProxy<State>;

/// Current state for the event loop
enum State {
	/// Start until first resume event
	Starting(Proxy),
	/// Wait until async is initialized
	Wait(Arc<winit::window::Window>, Proxy, bool),
	/// Main phase
	Running(Program),
}

type EventLoop = winit::event_loop::ActiveEventLoop;

impl winit::application::ApplicationHandler<State> for App {
	fn user_event(&mut self, _event_loop: &EventLoop, event: State) {
		self.state = event;
	}

	fn resumed(&mut self, event_loop: &EventLoop) {
		match self.state {
			State::Starting(ref proxy) => {
				let window = event_loop
					.create_window(
						winit::window::Window::default_attributes()
							.with_title("Treee")
							.with_min_inner_size(winit::dpi::LogicalSize { width: 10, height: 10 }),
					)
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
				self.state = State::Wait(window.clone(), proxy.clone(), false);
			},
			_ => {},
		}
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
		match self.state {
			State::Starting(_) => {
				log::error!("try in starting phase");
			},
			State::Wait(ref mut window, ref mut proxy, ref mut setup) => {
				// request redraw until async is initialized
				window.request_redraw();
				let size = window.inner_size();
				if *setup || size.width == 0 || size.height == 0 {
					return;
				}
				let window = window.clone();
				let proxy = proxy.clone();
				#[cfg(not(target_arch = "wasm32"))]
				{
					use pollster::FutureExt;
					let app = match Program::new(window).block_on() {
						Ok(program) => State::Running(program),
						Err(err) => return (self.error_handler)(err),
					};
					if let Err(_) = proxy.send_event(app) {
						return (self.error_handler)(Error::ProxyError);
					}
				}

				#[cfg(target_arch = "wasm32")]
				{
					let error_handler = self.error_handler;
					wasm_bindgen_futures::spawn_local(async move {
						let app = match Program::new(window).await {
							Ok(program) => State::Running(program),
							Err(err) => return error_handler(err),
						};
						if let Err(_) = proxy.send_event(app) {
							return error_handler(Error::ProxyError);
						}
					});
				}
				*setup = true;
			},
			State::Running(ref mut program) => match action(program) {
				Ok(()) => {},
				Err(err) => {
					event_loop.exit();
					(self.error_handler)(err);
				},
			},
		}
	}
}

/// Abstraction for OS specific interactions.
#[cfg(not(target_arch = "wasm32"))]
pub mod environment {
	use std::{fs::File, io::BufWriter};

	use super::*;

	pub struct Source {
		path: std::path::PathBuf,
	}

	impl Source {
		pub fn start(sender: &crossbeam::channel::Sender<Event>) {
			let path = rfd::FileDialog::new()
				.add_filter("Pointcloud", &["las", "laz", "ipc"])
				.pick_file();
			if let Some(path) = path {
				_ = sender.send(Event::Load(Self { path }));
			}
		}

		pub fn reader(&self) -> impl Read + Seek + '_ {
			std::io::BufReader::new(std::fs::File::open(&self.path).unwrap())
		}

		pub fn extension(&self) -> &str {
			self.path.extension().unwrap().to_str().unwrap()
		}
	}

	pub struct Saver {
		file: BufWriter<File>,
	}

	impl Saver {
		pub fn start(
			file_name: impl Into<String> + Send + 'static,
			extension: impl ToString + Send + 'static,
			action: impl FnOnce(Self) + Send + 'static,
		) {
			rayon::spawn(move || {
				let path = rfd::FileDialog::new()
					.set_file_name(file_name)
					.add_filter("", &[extension])
					.save_file();
				if let Some(path) = path {
					let file = BufWriter::new(File::create(path).unwrap());
					action(Self { file });
				}
			});
		}

		pub fn inner(&mut self) -> impl Write + '_ {
			&mut self.file
		}

		pub fn save(self) {}
	}
}

#[cfg(target_arch = "wasm32")]
pub mod environment {
	use super::*;

	pub struct Source {
		data: Vec<u8>,
		name: String,
	}

	impl Source {
		pub fn start(sender: &crossbeam::channel::Sender<Event>) {
			let sender = sender.clone();
			wasm_bindgen_futures::spawn_local(async move {
				let handle = rfd::AsyncFileDialog::new()
					.add_filter("Pointcloud", &["las", "laz", "ipc"])
					.pick_file()
					.await;
				if let Some(handle) = handle {
					let data = handle.read().await;
					let name = handle.file_name();
					_ = sender.send(Event::Load(Self { data, name }));
				}
			});
		}

		pub fn from_data(data: Vec<u8>, name: String) -> Self {
			Self { data, name }
		}

		pub fn reader<'a>(&'a self) -> impl Read + Seek + 'a {
			std::io::Cursor::new(&self.data)
		}

		pub fn extension(&self) -> &str {
			self.name.split(".").last().unwrap()
		}
	}

	pub struct Saver {
		handle: rfd::FileHandle,
		data: Vec<u8>,
	}

	impl Saver {
		pub fn start(
			file_name: impl Into<String> + Send + 'static,
			extension: impl ToString + Send + 'static,
			action: impl FnOnce(Saver) + Send + 'static,
		) {
			let name = format!("{}.{}", file_name.into(), extension.to_string());
			wasm_bindgen_futures::spawn_local(async move {
				let handle = rfd::AsyncFileDialog::new()
					.set_file_name(name)
					.save_file()
					.await;
				if let Some(handle) = handle {
					action(Self { data: Vec::new(), handle });
				}
			});
		}

		pub fn inner<'a>(&'a mut self) -> impl Write + 'a {
			&mut self.data
		}

		pub fn save(self) {
			wasm_bindgen_futures::spawn_local(async move {
				self.handle.write(&self.data).await.unwrap();
			});
		}
	}
}
