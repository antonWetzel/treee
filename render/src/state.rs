use math::Vector;
use winit::platform::run_return::EventLoopExtRunReturn;

use super::*;

pub struct State {
	pub(crate) device: wgpu::Device,
	pub(crate) queue: wgpu::Queue,
	pub(crate) instance: wgpu::Instance,
	pub(crate) adapter: wgpu::Adapter,
	pub(crate) surface_format: wgpu::TextureFormat,
}

impl State {
	pub async fn new() -> (Self, Runner) {
		env_logger::init();
		let event_loop = winit::event_loop::EventLoop::new();

		let window = winit::window::WindowBuilder::new()
			.with_visible(false)
			.build(&event_loop)
			.unwrap();

		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
			backends: wgpu::Backends::all(),
			dx12_shader_compiler: Default::default(),
		});

		let surface = unsafe { instance.create_surface(&window) }.unwrap();

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::default(),
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
			})
			.await
			.unwrap();

		let (device, queue) = adapter
			.request_device(
				&wgpu::DeviceDescriptor {
					features: wgpu::Features::empty(),
					limits: wgpu::Limits::default(),
					label: None,
				},
				None, // Trace path
			)
			.await
			.unwrap();

		let surface_caps = surface.get_capabilities(&adapter);
		let surface_format = surface_caps
			.formats
			.iter()
			.copied()
			.find(|f| f.describe().srgb)
			.unwrap_or(surface_caps.formats[0]);

		(
			Self {
				instance,
				adapter,
				device,
				queue,
				surface_format,
			},
			Runner { event_loop },
		)
	}
}

pub struct Runner {
	pub event_loop: winit::event_loop::EventLoop<()>,
}

impl Runner {
	pub fn run<T: Game>(mut self, game: &mut T) -> i32 {
		self.event_loop
			.run_return(|event, _event_loop, control_flow| {
				*control_flow = match event {
					winit::event::Event::WindowEvent { ref event, window_id } => match event {
						winit::event::WindowEvent::CloseRequested => game.close_window(window_id),
						winit::event::WindowEvent::Resized(size) => {
							game.resize_window(window_id, [size.width, size.height].into())
						},
						winit::event::WindowEvent::ScaleFactorChanged { .. } => todo!(),
						winit::event::WindowEvent::KeyboardInput { input, .. } => {
							let key = match input.virtual_keycode {
								Some(key) => key,
								None => return,
							};
							game.key_changed(window_id, key, input.state)
						},
						winit::event::WindowEvent::MouseInput { state: button_state, button, .. } => {
							game.mouse_pressed(window_id, (*button).into(), *button_state)
						},
						winit::event::WindowEvent::MouseWheel { delta, .. } => {
							let delta = match *delta {
								winit::event::MouseScrollDelta::LineDelta(_, y) => -y,
								winit::event::MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
							};
							game.mouse_wheel(delta)
						},
						winit::event::WindowEvent::CursorMoved { position, .. } => {
							let position = Vector::from([position.x, position.y]);
							game.mouse_moved(window_id, position)
						},
						&winit::event::WindowEvent::ModifiersChanged(modifiers) => {
							game.modifiers_changed(modifiers);
							ControlFlow::Poll
						},
						_ => ControlFlow::Poll,
					},
					winit::event::Event::RedrawRequested(window_id) => {
						game.render(window_id);
						ControlFlow::Poll
					},
					winit::event::Event::MainEventsCleared => game.time(),
					_ => ControlFlow::Poll,
				}
			})
	}
}
