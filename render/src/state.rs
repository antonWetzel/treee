use std::sync::Arc;

use super::*;

#[derive(Debug)]
pub struct State {
	pub device: wgpu::Device,
	pub queue: wgpu::Queue,
	pub surface_format: wgpu::TextureFormat,
}

#[derive(thiserror::Error, Debug)]
pub enum RenderError {
	#[error(transparent)]
	RenderError(#[from] winit::error::EventLoopError),

	#[error(transparent)]
	CreateSurfaceError(#[from] wgpu::CreateSurfaceError),

	#[error("No WebGPU support")]
	NoWebGPUSupport,

	#[error("Failed to get WebGPU device")]
	RequestDeviceError,
}

impl State {
	pub async fn new(window: Arc<winit::window::Window>) -> Result<(Self, Window), RenderError> {
		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
			#[cfg(not(feature = "webgl"))]
			backends: wgpu::Backends::PRIMARY,
			#[cfg(feature = "webgl")]
			backends: wgpu::Backends::GL,
			dx12_shader_compiler: wgpu::Dx12Compiler::default(),
			flags: wgpu::InstanceFlags::default(),
			gles_minor_version: wgpu::Gles3MinorVersion::default(),
		});

		let surface = instance.create_surface(window.clone())?;

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::HighPerformance,
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
			})
			.await
			.ok_or(RenderError::NoWebGPUSupport)?;

		let (device, queue) = adapter
			.request_device(
				&wgpu::DeviceDescriptor {
					required_features: wgpu::Features::empty(),
					#[cfg(not(feature = "webgl"))]
					required_limits: wgpu::Limits::default(),
					#[cfg(feature = "webgl")]
					required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
					label: None,
					memory_hints: wgpu::MemoryHints::Performance,
				},
				None,
			)
			.await
			.map_err(|_| RenderError::RequestDeviceError)?;

		let size = window.inner_size();
		let surface_caps = surface.get_capabilities(&adapter);

		let surface_format = surface_caps
			.formats
			.iter()
			.find(|&&format| format == wgpu::TextureFormat::Bgra8Unorm)
			.copied()
			.unwrap_or(surface_caps.formats[0]);
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
