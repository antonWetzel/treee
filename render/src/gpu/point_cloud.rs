use super::*;

pub struct PointCloud {
	pub buffer:    wgpu::Buffer,
	pub quad:      wgpu::Buffer,
	pub instances: u32,
}

impl PointCloud {
	pub fn new(state: &crate::State, vertices: &Vec<crate::Point>) -> Self {
		return Self::new_buffer(&state.device, vertices);
	}

	pub fn new_buffer(device: &wgpu::Device, vertices: &Vec<crate::Point>) -> Self {
		let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label:    Some("Vertex Buffer"),
			contents: bytemuck::cast_slice(&vertices[..]),
			usage:    wgpu::BufferUsages::VERTEX,
		});

		let quad_data = [
			crate::PointEdge { position: [-1.0, -1.0].into() },
			crate::PointEdge { position: [1.0, -1.0].into() },
			crate::PointEdge { position: [1.0, 1.0].into() },
			crate::PointEdge { position: [-1.0, -1.0].into() },
			crate::PointEdge { position: [1.0, 1.0].into() },
			crate::PointEdge { position: [-1.0, 1.0].into() },
		];
		let quad = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label:    Some("Quad Buffer"),
			contents: bytemuck::cast_slice(&quad_data),
			usage:    wgpu::BufferUsages::VERTEX,
		});
		Self {
			buffer,
			quad,
			instances: vertices.len() as u32,
		}
	}

	pub fn render<'c, 'b: 'c, 'a: 'b>(&'a self, render_pass: &'c mut crate::RenderPass<'b>) {
		render_pass.set_vertex_buffer(0, self.quad.slice(..));
		render_pass.set_vertex_buffer(1, self.buffer.slice(..));
		render_pass.draw(0..6, 0..self.instances);
	}
}
