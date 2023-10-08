use math::Vector;

use crate::eye_dome::EyeDome;

use super::*;

pub type WindowId = winit::window::WindowId;

pub struct Window {
	window: winit::window::Window,
	config: wgpu::SurfaceConfiguration,
	surface: wgpu::Surface,
	depth_texture: DepthTexture,
	middle_texture: wgpu::Texture,
	eye_dome: EyeDome,
}

impl Window {
	pub fn new<T: Into<String>>(
		state: &impl Has<State>,
		window_target: &winit::event_loop::EventLoopWindowTarget<()>,
		title: T,
	) -> Self {
		let state = state.get();
		let window = winit::window::WindowBuilder::new()
			.with_title(title)
			.with_min_inner_size(winit::dpi::LogicalSize { width: 10, height: 10 })
			.build(window_target)
			.unwrap();
		let size = window.inner_size();
		let surface = unsafe { state.instance.create_surface(&window) }.unwrap();
		let surface_caps = surface.get_capabilities(&state.adapter);
		let surface_format = *surface_caps
			.formats
			.iter()
			.find(|f| f.describe().srgb)
			.unwrap_or(&surface_caps.formats[0]);
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

		let middle_texture = state.device.create_texture(&wgpu::TextureDescriptor {
			label: Some("test"),
			size: wgpu::Extent3d {
				width: config.width,
				height: config.height,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: config.format,
			usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
			view_formats: &[],
		});

		let depth_texture = DepthTexture::new(&state.device, &config, "depth");
		let eye_dome = EyeDome::new(state, &config, &depth_texture, &middle_texture);

		Self {
			window,
			surface,
			depth_texture,
			config,
			eye_dome,
			middle_texture,
		}
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
		self.window.id()
	}
	pub fn request_redraw(&self) {
		self.window.request_redraw();
	}

	pub fn resized(&mut self, state: &impl Has<State>) {
		let state = state.get();
		let size = self.size();
		self.config.width = size.width;
		self.config.height = size.height;
		self.surface.configure(&state.device, &self.config);
		self.depth_texture = DepthTexture::new(&state.device, &self.config, "depth");
		self.middle_texture = state.device.create_texture(&wgpu::TextureDescriptor {
			label: Some("test"),
			size: wgpu::Extent3d {
				width: self.config.width,
				height: self.config.height,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: self.config.format,
			usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
			view_formats: &[],
		});
		self.eye_dome
			.update(state, &self.depth_texture, &self.middle_texture);
	}

	pub fn render<S: Has<State>>(
		&self,
		state: &'static S,
		pipeline: &Pipeline3D,
		cam: &Camera3DGPU,
		renderable: &impl Renderable<S>,
	) {
		let view = self
			.middle_texture
			.create_view(&wgpu::TextureViewDescriptor::default());

		let mut encoder = state
			.get()
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

		let output = self.surface.get_current_texture().unwrap();
		let view = output
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());
		self.eye_dome.render(&view, &mut encoder);
		state.get().queue.submit(Some(encoder.finish()));
		output.present();
	}
}
