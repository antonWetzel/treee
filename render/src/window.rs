use math::Vector;

use super::*;

pub type WindowId = winit::window::WindowId;

pub struct Window {
	pub(crate) window: winit::window::Window,
	pub(crate) config: wgpu::SurfaceConfiguration,
	pub(crate) surface: wgpu::Surface,
	pub(crate) depth_texture: DepthTexture,
}

impl Window {
	pub fn new<T: Into<String>>(
		state: &State,
		window_target: &winit::event_loop::EventLoopWindowTarget<()>,
		title: T,
	) -> Self {
		let window = winit::window::WindowBuilder::new()
			.with_title(title)
			.with_min_inner_size(winit::dpi::LogicalSize { width: 10, height: 10 })
			.build(window_target)
			.unwrap();
		let size = window.inner_size();
		let surface = unsafe { state.instance.create_surface(&window) }.unwrap();
		let surface_caps = surface.get_capabilities(&state.adapter);
		let surface_format = surface_caps
			.formats
			.iter()
			.copied()
			.find(|f| f.describe().srgb)
			.unwrap_or(surface_caps.formats[0]);
		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: surface_format,
			width: size.width,
			height: size.height,
			present_mode: surface_caps.present_modes[0],
			alpha_mode: surface_caps.alpha_modes[0],
			view_formats: vec![],
		};
		surface.configure(&state.device, &config);
		let depth_texture = DepthTexture::create_depth_texture(&state.device, &config, "depth_texture");
		return Self { window, surface, depth_texture, config };
	}

	pub fn get_aspect(&self) -> f32 {
		self.config.width as f32 / self.config.height as f32
	}
	pub fn get_size(&self) -> Vector<2, u32> {
		[self.config.width, self.config.height].into()
	}

	pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
		self.window.inner_size()
	}

	pub fn set_title(&self, title: String) {
		self.window.set_title(title.as_str())
	}

	pub fn id(&self) -> WindowId {
		return self.window.id();
	}
	pub fn request_redraw(&self) {
		self.window.request_redraw();
	}

	pub fn resized(&mut self, state: &State) {
		let size = self.size();
		self.config.width = size.width;
		self.config.height = size.height;
		self.surface.configure(&state.device, &self.config);
		self.depth_texture = DepthTexture::create_depth_texture(&state.device, &self.config, "depth_texture");
	}

	pub fn render<T>(&self, state: &State, pipeline: &Pipeline3D, cam: &gpu::Camera3D, renderable: &T)
	where
		T: Renderable,
	{
		let output = self.surface.get_current_texture().unwrap();
		let view = output
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());
		let mut encoder = state
			.device
			.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });
		{
			let desc: wgpu::RenderPassDescriptor = wgpu::RenderPassDescriptor {
				label: Some("Render Pass"),
				color_attachments: &[Some(wgpu::RenderPassColorAttachment {
					view: &view,
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 }),
						store: true,
					},
				})],
				depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
					view: &self.depth_texture.view,
					depth_ops: Some(wgpu::Operations {
						load: wgpu::LoadOp::Clear(1.0),
						store: true,
					}),
					stencil_ops: None,
				}),
			};
			let mut render_pass = encoder.begin_render_pass(&desc);
			pipeline.render(&mut render_pass, cam, renderable, state);
		}
		state.queue.submit(Some(encoder.finish()));
		output.present();
	}
}
