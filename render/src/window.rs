use std::{ops::Deref, sync::Arc};

use nalgebra as na;

use super::*;

pub type WindowId = winit::window::WindowId;

pub struct Window {
	window: Arc<winit::window::Window>,
	config: wgpu::SurfaceConfiguration,
	surface: wgpu::Surface<'static>,
	depth_texture: DepthTexture,
}

impl Deref for Window {
	type Target = winit::window::Window;

	fn deref(&self) -> &Self::Target {
		&self.window
	}
}

impl Window {
	pub fn new(
		window: Arc<winit::window::Window>,
		config: wgpu::SurfaceConfiguration,
		surface: wgpu::Surface<'static>,
		depth_texture: DepthTexture,
	) -> Self {
		Self { window, config, surface, depth_texture }
	}

	pub fn get_aspect(&self) -> f32 {
		self.config.width as f32 / self.config.height as f32
	}

	pub fn get_size(&self) -> na::Point2<f32> {
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
		let icon =
			winit::window::Icon::from_rgba(img.to_rgba8().into_vec(), img.width(), img.height())
				.unwrap();
		self.window.set_window_icon(Some(icon));
	}

	#[cfg(target_os = "windows")]
	pub fn set_taskbar_icon(&self, png: &[u8]) {
		use winit::platform::windows::WindowExtWindows;

		let img = image::load_from_memory(png).unwrap();
		let icon =
			winit::window::Icon::from_rgba(img.to_rgba8().into_vec(), img.width(), img.height())
				.unwrap();
		self.window.set_taskbar_icon(Some(icon));
	}

	pub fn resized(&mut self, state: &State) {
		let size = self.window.inner_size();
		self.config.width = size.width;
		self.config.height = size.height;
		self.surface.configure(&state.device, &self.config);
		self.depth_texture = DepthTexture::new(&state.device, &self.config, "depth");
	}

	pub fn render(&self, state: &State, render: impl for<'b> FnOnce(&'b mut RenderContext)) {
		let Some(output) = self.surface.get_current_texture().ok() else {
			return;
		};
		let view = output.texture.create_view(&Default::default());

		let encoder = state
			.device
			.create_command_encoder(&wgpu::CommandEncoderDescriptor {
				label: Some("Render Encoder"),
			});

		let mut context = RenderContext {
			encoder,
			view,
			depth_texture: &self.depth_texture.view,
		};

		render(&mut context);

		state.queue.submit(Some(context.encoder.finish()));
		output.present();
	}

	pub fn inner(&self) -> &winit::window::Window {
		&self.window
	}
}

// Window stayed open, but unresponsive. Just hide it.
impl Drop for Window {
	fn drop(&mut self) {
		self.window.set_visible(false);
	}
}

pub struct RenderContext<'a> {
	encoder: wgpu::CommandEncoder,
	view: wgpu::TextureView,
	depth_texture: &'a wgpu::TextureView,
}

impl<'a> RenderContext<'a> {
	pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
		&mut self.encoder
	}

	pub fn render_pass(&mut self, background: na::Point3<f32>) -> RenderPass {
		let render_pass =
			RenderPass::new(self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("Render Pass"),
				color_attachments: &[Some(wgpu::RenderPassColorAttachment {
					view: &self.view,
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color {
							r: background.x as f64,
							g: background.y as f64,
							b: background.z as f64,
							a: 1.0,
						}),
						store: wgpu::StoreOp::Store,
					},
				})],
				depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
					view: self.depth_texture,
					depth_ops: Some(wgpu::Operations {
						load: wgpu::LoadOp::Clear(1.0),
						store: wgpu::StoreOp::Store,
					}),
					stencil_ops: None,
				}),
				occlusion_query_set: None,
				timestamp_writes: None,
			}));
		render_pass
	}

	pub fn post_process_pass(&mut self) -> RenderPass {
		let render_pass =
			RenderPass::new(self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("eye dome"),
				color_attachments: &[Some(wgpu::RenderPassColorAttachment {
					view: &self.view,
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
		render_pass
	}
}
