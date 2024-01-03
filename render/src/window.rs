use std::path::PathBuf;

use math::{ Vector, X, Y, Z };

use super::*;


pub type WindowId = winit::window::WindowId;
pub type EventResponse = egui_winit::EventResponse;


pub struct Window {
	pub window: winit::window::Window,
	config: wgpu::SurfaceConfiguration,
	surface: wgpu::Surface,
	depth_texture: DepthTexture,

	pub egui_winit: egui_winit::State,
	egui_wgpu: egui_wgpu::renderer::Renderer,
}


impl Window {
	pub fn new<T: Into<String>>(
		state: &impl Has<State>,
		window_target: &winit::event_loop::EventLoopWindowTarget<()>,
		title: T,
		egui: &egui::Context,
	) -> Self {
		let state = state.get();
		let window = winit::window::WindowBuilder::new()
			.with_title(title)
			.with_min_inner_size(winit::dpi::LogicalSize { width: 10, height: 10 })
			.build(window_target)
			.unwrap();
		let size = window.inner_size();
		let surface = unsafe {
			state.instance.create_surface(&window)
		}.unwrap();
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
			view_formats: Vec::new(),
		};
		surface.configure(&state.device, &config);

		let depth_texture = DepthTexture::new(&state.device, &config, "depth");

		let egui_wgpu = egui_wgpu::renderer::Renderer::new(
			&state.device,
			config.format,
			None,
			1,
		);

		let id = egui.viewport_id();
		Self {
			surface,
			depth_texture,
			config,

			egui_winit: egui_winit::State::new(
				egui.clone(),
				id,
				&window,
				None,
				None,
			),
			egui_wgpu,

			window,
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


	pub fn depth_texture(&self) -> &DepthTexture {
		&self.depth_texture
	}


	pub fn window_event(&mut self, event: &winit::event::WindowEvent) -> EventResponse {
		self.egui_winit.on_window_event(&self.window, event)
	}


	pub fn set_window_icon(&self, png: &[u8]) {
		let img = image::load_from_memory(png).unwrap();
		let icon = winit::window::Icon::from_rgba(img.to_rgba8().into_vec(), img.width(), img.height()).unwrap();
		self.window.set_window_icon(Some(icon));
	}


	#[cfg(target_os = "windows")]
	pub fn set_taskbar_icon(&self, png: &[u8]) {
		use winit::platform::windows::WindowExtWindows;


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


	pub fn screen_shot<S: Has<State>>(&mut self, state: &'static S, renderable: &mut impl RenderEntry<S>, path: PathBuf) {
		fn ceil_to_multiple(value: u32, base: u32) -> u32 {
			(value + (base - 1)) / base * base
		}


		let render_state: &State = state.get();
		let (texture_width, texture_height, format) = {
			let output = &self.surface.get_current_texture().unwrap().texture;

			(output.size().width, output.size().height, output.format())
		};

		let mut encoder = render_state
			.device
			.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });

		let texture_desc = wgpu::TextureDescriptor {
			size: wgpu::Extent3d {
				width: texture_width,
				height: texture_height,
				depth_or_array_layers: 1,
			},
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
			label: None,
			view_formats: &Vec::new(),
		};
		let texture = render_state.device.create_texture(&texture_desc);
		let view = texture.create_view(&Default::default());

		self.render_to(state, renderable, &view, renderable.background(), 0.0, (&[], &[], &[]));

		let u32_size = std::mem::size_of::<u32>() as u32;
		let texture_width = ceil_to_multiple(texture_width, 256 / 4);
		let buffer_size = (u32_size * texture_width * texture_height) as wgpu::BufferAddress;

		let buffer = render_state.device.create_buffer(&wgpu::BufferDescriptor {
			size: buffer_size,
			usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
			label: None,
			mapped_at_creation: false,
		});

		encoder.copy_texture_to_buffer(
			wgpu::ImageCopyTexture {
				aspect: wgpu::TextureAspect::All,
				texture: &texture,
				mip_level: 0,
				origin: wgpu::Origin3d::ZERO,
			},
			wgpu::ImageCopyBuffer {
				buffer: &buffer,
				layout: wgpu::ImageDataLayout {
					offset: 0,
					bytes_per_row: Some(ceil_to_multiple(
						u32_size * texture_width,
						wgpu::COPY_BYTES_PER_ROW_ALIGNMENT,
					)),
					rows_per_image: Some(texture_height),
				},
			},
			texture.size(),
		);
		render_state.queue.submit(Some(encoder.finish()));

		let buffer_slice = buffer.slice(..);

		let (tx, rx) = std::sync::mpsc::channel::<wgpu::Buffer>();
		buffer_slice.map_async(wgpu::MapMode::Read, move |_result| {
			let buffer = rx.recv().unwrap();
			let data = buffer.slice(..).get_mapped_range();
			let mut image = image::RgbaImage::from_raw(
				texture_width,
				texture_height,
				data.iter().copied().collect(),
			)
				.unwrap();
			drop(data);

			for pixel in image.pixels_mut() {
				pixel.0.swap(0, 2); // hack because input uses BGRA, not RGBA
			}
			image.save(path).unwrap();
			buffer.unmap();
		});
		tx.send(buffer).unwrap();
	}


	fn render_to<S: Has<State>>(
		&mut self,
		state: &'static S,
		renderable: &mut impl RenderEntry<S>,
		view: &wgpu::TextureView,
		background: Vector<3, f32>,
		alpha: f32,

		ui: (
			&[egui::ClippedPrimitive],
			&[(egui::TextureId, egui::epaint::ImageDelta)],
			&[egui::TextureId],
		),
	) -> f32 {
		let render_state = state.get();
		let set = render_state
			.device
			.create_query_set(&wgpu::QuerySetDescriptor {
				label: None,
				ty: wgpu::QueryType::Timestamp,
				count: 2,
			});
		let mut encoder = render_state
			.device
			.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });
		encoder.write_timestamp(&set, 0);
		let mut render_pass = RenderPass::new(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color {
						r: background[X] as f64,
						g: background[Y] as f64,
						b: background[Z] as f64,
						a: alpha as f64,
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

		renderable.render(state, &mut render_pass);
		drop(render_pass);

		let size = self.window.inner_size();
		let screen = &egui_wgpu::renderer::ScreenDescriptor {
			size_in_pixels: [size.width, size.height],
			pixels_per_point: 1.0,
		};
		for (id, delta) in ui.1 {
			self.egui_wgpu.update_texture(&render_state.device, &render_state.queue, *id, delta);
		}
		for id in ui.2 {
			self.egui_wgpu.free_texture(id);
		}
		let commands = self.egui_wgpu.update_buffers(&render_state.device, &render_state.queue, &mut encoder, ui.0, screen);
		render_state.queue.submit(commands);

		let mut render_pass = RenderPass::new(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("eye dome"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view,
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
		renderable.post_process(state, &mut render_pass);

		self.egui_wgpu.render(&mut render_pass, ui.0, screen);

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
		{
			map_buffer.slice(..).map_async(wgpu::MapMode::Read, |_| { });

			render_state.device.poll(wgpu::Maintain::Wait);

			let data = map_buffer.slice(..).get_mapped_range();
			let data = bytemuck::cast_slice::<u8, u64>(&data);
			let diff = data[1] - data[0];
			diff as f32 * 1e-9 * render_state.queue.get_timestamp_period()
		}
	}


	pub fn render<S: Has<State>>(&mut self, state: &'static S, renderable: &mut impl RenderEntry<S>, ui: egui::FullOutput, egui: &egui::Context) -> Option<f32> {
		let output = self.surface.get_current_texture().ok()?;
		let view = output
			.texture
			.create_view(&Default::default());

		self.egui_winit.handle_platform_output(&self.window, ui.platform_output);
		let paint_jobs = egui.tessellate(ui.shapes, ui.pixels_per_point);

		let res = self.render_to(state, renderable, &view, renderable.background(), 1.0, (&paint_jobs, &ui.textures_delta.set, &ui.textures_delta.free));
		output.present();
		Some(res)
	}
}
