use wgpu::util::DeviceExt;

use crate::{Has, RenderPass, State};

pub struct PointCloudState {
	pub quad: wgpu::Buffer,
}

impl PointCloudState {
	pub fn new(state: &State) -> Self {
		let quad_data = [
			crate::PointEdge { position: [-1.0, -1.0].into() },
			crate::PointEdge { position: [1.0, -1.0].into() },
			crate::PointEdge { position: [1.0, 1.0].into() },
			crate::PointEdge { position: [-1.0, -1.0].into() },
			crate::PointEdge { position: [1.0, 1.0].into() },
			crate::PointEdge { position: [-1.0, 1.0].into() },
		];
		let quad = state
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Quad Buffer"),
				contents: bytemuck::cast_slice(&quad_data),
				usage: wgpu::BufferUsages::VERTEX,
			});
		Self { quad }
	}

	pub fn activate<'a, 'b>(&'a self, render_pass: &'b mut RenderPass<'a>) -> &'b mut PointCloudPass<'a> {
		render_pass.set_vertex_buffer(0, self.quad.slice(..));
		unsafe { std::mem::transmute(render_pass) }
	}
}

// type PointCloudPass<'a> = RenderPass<'a>;
#[repr(transparent)]
pub struct PointCloudPass<'a>(wgpu::RenderPass<'a>);

impl<'a> std::ops::Deref for PointCloudPass<'a> {
	type Target = wgpu::RenderPass<'a>;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<'a> std::ops::DerefMut for PointCloudPass<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

pub struct PointCloud {
	pub buffer: wgpu::Buffer,
	pub instances: u32,
}

impl PointCloud {
	pub fn new(state: &impl Has<State>, vertices: &Vec<crate::Point>) -> Self {
		let buffer = state
			.get()
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Vertex Buffer"),
				contents: bytemuck::cast_slice(&vertices[..]),
				usage: wgpu::BufferUsages::VERTEX,
			});

		Self { buffer, instances: vertices.len() as u32 }
	}

	pub fn render<'a>(&'a self, render_pass: &mut PointCloudPass<'a>) {
		render_pass.set_vertex_buffer(1, self.buffer.slice(..));
		render_pass.draw(0..6, 0..self.instances);
	}
}
