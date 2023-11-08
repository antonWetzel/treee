use math::Vector;

use super::*;

pub type WindowId = winit::window::WindowId;

pub struct Window {
	window: winit::window::Window,
	config: wgpu::SurfaceConfiguration,
	surface: wgpu::Surface,

	depth_texture_left: DepthTexture,
	depth_texture_right: DepthTexture,

	left: wgpu::Texture,
	right: wgpu::Texture,
}

impl Window {
	pub fn new<T: Into<String>>(
		state: &impl Has<State>,
		window_target: &winit::event_loop::EventLoopWindowTarget<()>,
		title: T,
	) -> Self {
		let state: &State = state.get();
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
			.find(|f| f.is_srgb())
			.unwrap_or(&surface_caps.formats[0]);
		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
			format: surface_format,
			width: size.width,
			height: size.height,
			present_mode: surface_caps.present_modes[0],
			alpha_mode: surface_caps.alpha_modes[0],
			view_formats: vec![],
		};
		surface.configure(&state.device, &config);

		let left = state.device.create_texture(&wgpu::TextureDescriptor {
			label: None,
			size: wgpu::Extent3d {
				width: (size.width + 1) / 2,
				height: size.height,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: surface_format,
			view_formats: &[],
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
		});

		let right = state.device.create_texture(&wgpu::TextureDescriptor {
			label: None,
			size: wgpu::Extent3d {
				width: size.width / 2,
				height: size.height,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: surface_format,
			view_formats: &[],
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
		});

		Self {
			window,
			surface,
			depth_texture_left: DepthTexture::new(&state.device, left.size(), "depth left"),
			depth_texture_right: DepthTexture::new(&state.device, right.size(), "depth right"),
			config,
			left,
			right,
		}
	}

	pub fn get_aspect(&self) -> f32 {
		self.config.width as f32 / self.config.height as f32
	}
	pub fn get_size(&self) -> Vector<2, f32> {
		[self.config.width as f32, self.config.height as f32].into()
	}

	pub fn set_title(&self, title: &str) {
		self.window.set_title(title)
	}

	pub fn id(&self) -> WindowId {
		self.window.id()
	}
	pub fn request_redraw(&self) {
		self.window.request_redraw();
	}

	pub fn config(&self) -> &wgpu::SurfaceConfiguration {
		&self.config
	}

	// pub fn depth_texture(&self) -> &DepthTexture {
	// 	&self.depth_texture
	// }

	pub fn resized(&mut self, state: &impl Has<State>) {
		let state = state.get();
		let size = self.window.inner_size();
		self.config.width = size.width;
		self.config.height = size.height;
		self.surface.configure(&state.device, &self.config);

		self.left = state.device.create_texture(&wgpu::TextureDescriptor {
			label: None,
			size: wgpu::Extent3d {
				width: (size.width + 1) / 2,
				height: size.height,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: self.config.format,
			view_formats: &[],
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
		});

		self.right = state.device.create_texture(&wgpu::TextureDescriptor {
			label: None,
			size: wgpu::Extent3d {
				width: size.width / 2,
				height: size.height,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: self.config.format,
			view_formats: &[],
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
		});

		self.depth_texture_left = DepthTexture::new(&state.device, self.left.size(), "depth left");
		self.depth_texture_right = DepthTexture::new(&state.device, self.right.size(), "depth right");
	}

	pub fn render(&self, state: &'static impl Has<State>, renderable: &impl RenderEntry) {
		let render_state: &State = state.get();
		let output = self.surface.get_current_texture().unwrap();

		let view = output
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());

		let mut encoder = render_state
			.device
			.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

		for (side, texture, depth_texture) in [
			(false, &self.left, &self.depth_texture_left),
			(true, &self.right, &self.depth_texture_right),
		] {
			let texture_view = texture.create_view(&Default::default());
			let mut render_pass = RenderPass::new(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("Render Pass"),
				color_attachments: &[Some(wgpu::RenderPassColorAttachment {
					view: &texture_view,
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 }),
						store: wgpu::StoreOp::Store,
					},
				})],
				depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
					view: &depth_texture.view,
					depth_ops: Some(wgpu::Operations {
						load: wgpu::LoadOp::Clear(1.0),
						store: wgpu::StoreOp::Store,
					}),
					stencil_ops: None,
				}),
				occlusion_query_set: None,
				timestamp_writes: None,
			}));

			renderable.render(&mut render_pass, side);
			drop(render_pass);

			let origin = if side {
				wgpu::Origin3d { x: texture.size().width, y: 0, z: 0 }
			} else {
				wgpu::Origin3d { x: 0, y: 0, z: 0 }
			};
			encoder.copy_texture_to_texture(
				texture.as_image_copy(),
				wgpu::ImageCopyTexture {
					texture: &output.texture,
					mip_level: 0,
					origin,
					aspect: wgpu::TextureAspect::All,
				},
				texture.size(),
			);
		}

		let mut render_pass = RenderPass::new(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("post process"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Load,
					store: wgpu::StoreOp::Store,
				},
			})],
			depth_stencil_attachment: None,
			occlusion_query_set: None,
			timestamp_writes: None,
		}));
		renderable.post_process(&mut render_pass);
		drop(render_pass);

		render_state.queue.submit(Some(encoder.finish()));
		output.present();
	}
}
