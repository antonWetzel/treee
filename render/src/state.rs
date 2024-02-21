use std::sync::Arc;

use math::Vector;

use super::*;

pub struct State {
	pub(crate) device: wgpu::Device,
	pub(crate) queue: wgpu::Queue,
	pub(crate) surface_format: wgpu::TextureFormat,
}

impl Has<Self> for State {
	fn get(&self) -> &Self {
		self
	}
}

pub type RenderError = winit::error::EventLoopError;

impl State {
	pub async fn new(title: &str, egui: &egui::Context) -> Result<(Self, Window, Runner), RenderError> {
		let event_loop = winit::event_loop::EventLoop::new()?;

		let window = winit::window::WindowBuilder::new()
			.with_visible(false)
			.build(&event_loop)
			.unwrap();

		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
			backends: wgpu::Backends::PRIMARY,
			dx12_shader_compiler: wgpu::Dx12Compiler::default(),
			flags: wgpu::InstanceFlags::default(),
			gles_minor_version: wgpu::Gles3MinorVersion::default(),
		});

		let surface = instance.create_surface(&window).unwrap();

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::HighPerformance,
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
			})
			.await
			.unwrap();

		let (device, queue) = adapter
			.request_device(
				&wgpu::DeviceDescriptor {
					required_features: wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::POLYGON_MODE_LINE,
					required_limits: wgpu::Limits {
						// max_buffer_size: u64::MAX,
						..Default::default()
					},
					label: None,
				},
				None,
			)
			.await
			.unwrap();

		let window = winit::window::WindowBuilder::new()
			.with_title(title)
			.with_min_inner_size(winit::dpi::LogicalSize { width: 10, height: 10 })
			.build(&event_loop)
			.unwrap();
		let window = Arc::new(window);

		let size = window.inner_size();
		let surface = instance.create_surface(window.clone()).unwrap();
		let surface_caps = surface.get_capabilities(&adapter);
		let surface_format = *surface_caps
			.formats
			.iter()
			.find(|f| f.is_srgb())
			.unwrap_or(&surface_caps.formats[0]);
		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: surface_format,
			width: size.width,
			height: size.height,
			present_mode: surface_caps.present_modes[0],
			alpha_mode: surface_caps.alpha_modes[0],
			desired_maximum_frame_latency: 2,
			view_formats: Vec::new(),
		};
		surface.configure(&device, &config);

		let depth_texture = DepthTexture::new(&device, &config, "depth");
		let id = egui.viewport_id();
		let egui_wgpu = egui_wgpu::Renderer::new(&device, config.format, None, 1);
		let egui_winit = egui_winit::State::new(egui.clone(), id, &window, None, None);

		let window = Window {
			surface,
			depth_texture,
			config,

			egui_wgpu,
			egui_winit,

			window,
		};

		Ok((
			Self { device, queue, surface_format },
			window,
			Runner { event_loop },
		))
	}
}

pub struct Runner {
	pub event_loop: winit::event_loop::EventLoop<()>,
}

impl Runner {
	pub fn run<T: Entry>(self, game: &mut T) -> Result<(), RenderError> {
		self.event_loop
			.set_control_flow(winit::event_loop::ControlFlow::Poll);
		self.event_loop.run(|event, event_loop| {
			match event {
				winit::event::Event::WindowEvent { event, window_id } => {
					if game.raw_event(&event) {
						return;
					}
					match event {
						winit::event::WindowEvent::CloseRequested => game.close_window(window_id),
						winit::event::WindowEvent::Resized(size) => {
							game.resize_window(window_id, [size.width, size.height].into())
						},
						winit::event::WindowEvent::ScaleFactorChanged { .. } => todo!(),
						winit::event::WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
							winit::keyboard::PhysicalKey::Code(key) => game.key_changed(window_id, key, event.state),
							winit::keyboard::PhysicalKey::Unidentified(_) => {},
						},
						winit::event::WindowEvent::MouseInput { state: button_state, button, .. } => {
							game.mouse_button_changed(window_id, (button).into(), button_state)
						},
						winit::event::WindowEvent::MouseWheel { delta, .. } => {
							let delta = match delta {
								winit::event::MouseScrollDelta::LineDelta(_, y) => -y,
								winit::event::MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
							};
							game.mouse_wheel(delta)
						},
						winit::event::WindowEvent::CursorMoved { position, .. } => {
							let position = Vector::from([position.x as f32, position.y as f32]);
							game.mouse_moved(window_id, position)
						},
						winit::event::WindowEvent::ModifiersChanged(modifiers) => {
							game.modifiers_changed(modifiers.state())
						},
						winit::event::WindowEvent::RedrawRequested => game.render(window_id),
						_ => {},
					}
				},
				winit::event::Event::AboutToWait => game.time(),
				_ => {},
			}

			if game.exit() {
				event_loop.exit();
			}
		})
	}
}
