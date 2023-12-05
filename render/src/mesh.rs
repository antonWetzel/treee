use wgpu::util::DeviceExt;

use crate::{
	depth_texture::DepthTexture, Camera3DGPU, Has, Lookup, Point, PointCloud, PointCloudProperty, Render, RenderPass,
	State,
};

pub struct MeshState {
	pipeline: wgpu::RenderPipeline,
}

impl MeshState {
	pub fn new(state: &impl Has<State>) -> Self {
		let state = state.get();

		let shader = state
			.device
			.create_shader_module(wgpu::include_wgsl!("mesh.wgsl"));
		let render_pipeline_layout = state
			.device
			.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("Render Pipeline Layout"),
				bind_group_layouts: &[&Camera3DGPU::get_layout(state), &Lookup::get_layout(state)],
				push_constant_ranges: &[],
			});

		let pipeline = state
			.device
			.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
				label: Some("mesh"),
				layout: Some(&render_pipeline_layout),
				vertex: wgpu::VertexState {
					module: &shader,
					entry_point: "vs_main",
					buffers: &[
						Point::description(wgpu::VertexStepMode::Vertex),
						Point::property_description(wgpu::VertexStepMode::Vertex),
					],
				},
				fragment: Some(wgpu::FragmentState {
					module: &shader,
					entry_point: "fs_main",
					targets: &[Some(wgpu::ColorTargetState {
						format: state.surface_format,
						blend: Some(wgpu::BlendState::REPLACE),
						write_mask: wgpu::ColorWrites::ALL,
					})],
				}),
				primitive: wgpu::PrimitiveState {
					topology: wgpu::PrimitiveTopology::TriangleList,
					strip_index_format: None,
					front_face: wgpu::FrontFace::Ccw,
					cull_mode: None,
					polygon_mode: wgpu::PolygonMode::Fill,
					unclipped_depth: false,
					conservative: false,
				},
				depth_stencil: Some(wgpu::DepthStencilState {
					format: DepthTexture::DEPTH_FORMAT,
					depth_write_enabled: true,
					depth_compare: wgpu::CompareFunction::Less,
					stencil: wgpu::StencilState::default(),
					bias: wgpu::DepthBiasState::default(),
				}),
				multisample: wgpu::MultisampleState {
					count: 1,
					mask: !0,
					alpha_to_coverage_enabled: false,
				},
				multiview: None,
			});

		Self { pipeline }
	}
}

#[repr(transparent)]
pub struct MeshPass<'a>(wgpu::RenderPass<'a>);

pub trait MeshRender {
	fn render<'a>(&'a self, mesh_pass: &mut MeshPass<'a>);
}

impl<'a, T, S: Has<MeshState>> Render<'a, (&'a S, &'a Camera3DGPU, &'a Lookup)> for T
where
	T: MeshRender,
{
	fn render(&'a self, render_pass: &mut RenderPass<'a>, data: (&'a S, &'a Camera3DGPU, &'a Lookup)) {
		let (state, camera, lookup) = (data.0.get(), data.1, data.2);
		render_pass.set_pipeline(&state.pipeline);
		render_pass.set_bind_group(0, camera.get_bind_group(), &[]);
		render_pass.set_bind_group(1, lookup.get_bind_group(), &[]);
		let mesh_pass = unsafe { std::mem::transmute::<_, &mut MeshPass<'a>>(render_pass) };
		self.render(mesh_pass);
	}
}

#[derive(Debug)]
pub struct Mesh {
	pub buffer: wgpu::Buffer,
	pub instances: u32,
}

impl Mesh {
	pub fn new(state: &impl Has<State>, indices: &[u32]) -> Self {
		let buffer = state
			.get()
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("mesh buffer"),
				contents: bytemuck::cast_slice(indices),
				usage: wgpu::BufferUsages::INDEX,
			});

		Self { buffer, instances: indices.len() as u32 }
	}

	pub fn render<'a>(
		&'a self,
		mesh_pass: &mut MeshPass<'a>,
		point_cloud: &'a PointCloud,
		property: &'a PointCloudProperty,
	) {
		mesh_pass
			.0
			.set_vertex_buffer(0, point_cloud.buffer.slice(..));
		mesh_pass.0.set_vertex_buffer(
			1,
			property
				.buffer
				.slice(0..(point_cloud.instances * std::mem::size_of::<u32>() as u32) as wgpu::BufferAddress),
		);
		mesh_pass
			.0
			.set_index_buffer(self.buffer.slice(..), wgpu::IndexFormat::Uint32);
		mesh_pass.0.draw_indexed(0..self.instances as u32, 0, 0..1);
	}
}
