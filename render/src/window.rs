use math::{Vector, X, Y, Z};
use winit::platform::windows::WindowExtWindows;

use super::*;

pub type WindowId = winit::window::WindowId;

pub struct Window {
	window: winit::window::Window,
	config: wgpu::SurfaceConfiguration,
	surface: wgpu::Surface,
	depth_texture: DepthTexture,
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
			.find(|f| f.is_srgb())
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

		let depth_texture = DepthTexture::new(&state.device, &config, "depth");

		Self { window, surface, depth_texture, config }
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

	pub fn depth_texture(&self) -> &DepthTexture {
		&self.depth_texture
	}

	pub fn set_window_icon(&self, png: &[u8]) {
		let img = image::load_from_memory(png).unwrap();
		let icon = winit::window::Icon::from_rgba(img.to_rgba8().into_vec(), img.width(), img.height()).unwrap();
		self.window.set_window_icon(Some(icon));
	}

	#[cfg(target_os = "windows")]
	pub fn set_taskbar_icon(&self, png: &[u8]) {
		let img = image::load_from_memory(png).unwrap();
		let icon = winit::window::Icon::from_rgba(img.to_rgba8().into_vec(), img.width(), img.height()).unwrap();
		self.window.set_taskbar_icon(Some(icon));
	}

	pub fn resized(&mut self, state: &impl Has<State>) {
		let state = state.get();
		let size = self.window.inner_size();
		self.config.width = size.width;
		self.config.height = size.height;
		self.surface.configure(&state.device, &self.config);
		self.depth_texture = DepthTexture::new(&state.device, &self.config, "depth");
	}

	pub fn render(&self, state: &'static impl Has<State>, renderable: &impl RenderEntry) -> f32 {
		let render_state: &State = state.get();
		let output = self.surface.get_current_texture().unwrap();
		let view = output
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());

		let set = render_state
			.device
			.create_query_set(&wgpu::QuerySetDescriptor {
				label: None,
				ty: wgpu::QueryType::Timestamp,
				count: 2,
			});
		let background = renderable.background();
		let mut encoder = render_state
			.device
			.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });
		encoder.write_timestamp(&set, 0);
		let mut render_pass = RenderPass::new(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color {
						r: background[X] as f64,
						g: background[Y] as f64,
						b: background[Z] as f64,
						a: 1.0,
					}),
					store: wgpu::StoreOp::Store,
				},
			})],
			depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
				view: &self.depth_texture.view,
				depth_ops: Some(wgpu::Operations {
					load: wgpu::LoadOp::Clear(1.0),
					store: wgpu::StoreOp::Store,
				}),
				stencil_ops: None,
			}),
			occlusion_query_set: None,
			timestamp_writes: None,
		}));

		renderable.render(&mut render_pass);
		drop(render_pass);

		let view = output
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());
		let mut render_pass = RenderPass::new(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("eye dome"),
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

		let buffer = render_state.device.create_buffer(&wgpu::BufferDescriptor {
			label: None,
			mapped_at_creation: false,
			size: 8 * 2,
			usage: wgpu::BufferUsages::QUERY_RESOLVE
				| wgpu::BufferUsages::STORAGE
				| wgpu::BufferUsages::COPY_SRC
				| wgpu::BufferUsages::COPY_DST,
		});

		encoder.write_timestamp(&set, 1);
		encoder.resolve_query_set(&set, 0..2, &buffer, 0);

		let map_buffer = render_state.device.create_buffer(&wgpu::BufferDescriptor {
			label: None,
			mapped_at_creation: false,
			size: 8 * 2,
			usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
		});
		encoder.copy_buffer_to_buffer(&buffer, 0, &map_buffer, 0, 8 * 2);

		render_state.queue.submit(Some(encoder.finish()));
		output.present();

		{
			map_buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});

			render_state.device.poll(wgpu::Maintain::Wait);

			let data = map_buffer.slice(..).get_mapped_range();
			let data = bytemuck::cast_slice::<u8, u64>(&data);
			let diff = data[1] - data[0];
			diff as f32 * 1e-9 * render_state.queue.get_timestamp_period()
		}
	}
}
