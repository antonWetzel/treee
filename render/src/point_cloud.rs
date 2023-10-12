use wgpu::util::DeviceExt;

use crate::{Has, Lookup, RenderPass, State};

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

	pub fn activate<'a>(&'a self, mut render_pass: RenderPass<'a>, lookup: &'a Lookup) -> PointCloudPass<'a> {
		render_pass.set_vertex_buffer(0, self.quad.slice(..));
		render_pass.set_bind_group(1, lookup.get_bind_group(), &[]);
		PointCloudPass(render_pass)
	}
}

#[repr(transparent)]
pub struct PointCloudPass<'a>(wgpu::RenderPass<'a>);

impl<'a> PointCloudPass<'a> {
	pub fn stop(self) -> RenderPass<'a> {
		self.0
	}
}

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

#[derive(Debug)]
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

	pub fn render<'a>(&'a self, point_cloud_pass: &mut PointCloudPass<'a>) {
		point_cloud_pass.set_vertex_buffer(1, self.buffer.slice(..));
		point_cloud_pass.draw(0..6, 0..self.instances);
	}
}
