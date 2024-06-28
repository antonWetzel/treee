use std::sync::Arc;

use super::*;

#[derive(Debug)]
pub struct State {
	pub device: wgpu::Device,
	pub queue: wgpu::Queue,
	pub surface_format: wgpu::TextureFormat,
}

pub type RenderError = winit::error::EventLoopError;

impl State {
	pub async fn new(
		title: &str,
		event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
	) -> Result<(Self, Window), RenderError> {
		let window = winit::window::WindowBuilder::new()
			.with_title(title)
			.with_min_inner_size(winit::dpi::LogicalSize { width: 10, height: 10 })
			.build(event_loop)?;
		let window = Arc::new(window);

		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
			backends: wgpu::Backends::PRIMARY,
			dx12_shader_compiler: wgpu::Dx12Compiler::default(),
			flags: wgpu::InstanceFlags::default(),
			gles_minor_version: wgpu::Gles3MinorVersion::default(),
		});

		let surface = instance.create_surface(window.clone()).unwrap();

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
					required_features: wgpu::Features::POLYGON_MODE_LINE,
					required_limits: wgpu::Limits::default(),
					label: None,
				},
				None,
			)
			.await
			.unwrap();

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

		let window = Window::new(window, config, surface, depth_texture);

		Ok((Self { device, queue, surface_format }, window))
	}

	pub fn device(&self) -> &wgpu::Device {
		&self.device
	}

	pub fn queue(&self) -> &wgpu::Queue {
		&self.queue
	}

	pub fn surface_format(&self) -> wgpu::TextureFormat {
		self.surface_format
	}
}
