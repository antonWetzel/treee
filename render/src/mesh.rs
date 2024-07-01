use wgpu::util::DeviceExt;

use crate::{
	depth_texture::DepthTexture, point_description, point_property_description, Camera3DGPU, Lookup, PointCloud,
	PointCloudProperty, RenderPass, State,
};

pub struct MeshState {
	pipeline: wgpu::RenderPipeline,
}

impl MeshState {
	pub fn new(state: &State) -> Self {
		Self {
			pipeline: Self::create_pipeline(state, wgpu::PolygonMode::Fill, true),
		}
	}

	pub fn new_as_lines(state: &State) -> Self {
		Self {
			pipeline: Self::create_pipeline(state, wgpu::PolygonMode::Line, false),
		}
	}

	fn create_pipeline(state: &State, mode: wgpu::PolygonMode, cull: bool) -> wgpu::RenderPipeline {
		let shader = state
			.device
			.create_shader_module(wgpu::include_wgsl!("mesh.wgsl"));
		let render_pipeline_layout = state
			.device
			.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("Render Pipeline Layout"),
				// bind_group_layouts: &[&Camera3DGPU::get_layout(state), &Lookup::get_layout(state)],
				bind_group_layouts: &[&Camera3DGPU::get_layout(state)],
				push_constant_ranges: &[],
			});

		state
			.device
			.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
				label: Some("mesh"),
				layout: Some(&render_pipeline_layout),
				vertex: wgpu::VertexState {
					module: &shader,
					entry_point: "vs_main",
					buffers: &[
						point_description(wgpu::VertexStepMode::Vertex),
						// point_property_description(wgpu::VertexStepMode::Vertex),
					],
				},
				fragment: Some(wgpu::FragmentState {
					module: &shader,
					entry_point: "fs_main",
					targets: &[Some(wgpu::ColorTargetState {
						format: state.surface_format,
						// blend: Some(wgpu::BlendState::ALPHA_BLENDING),
						blend: Some(wgpu::BlendState::REPLACE),
						write_mask: wgpu::ColorWrites::ALL,
					})],
				}),
				primitive: wgpu::PrimitiveState {
					topology: wgpu::PrimitiveTopology::TriangleList,
					strip_index_format: None,
					front_face: wgpu::FrontFace::Ccw,
					cull_mode: cull.then_some(wgpu::Face::Front),
					polygon_mode: mode,
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
			})
	}

	pub fn render<'a, 'b>(
		&'a self,
		render_pass: &'b mut RenderPass<'a>,
		camera: &'a Camera3DGPU,
		// lookup: &'a Lookup,
	) -> &'b mut MeshPass<'a> {
		render_pass.set_pipeline(&self.pipeline);
		render_pass.set_bind_group(0, camera.get_bind_group(), &[]);
		// render_pass.set_bind_group(1, lookup.get_bind_group(), &[]);
		unsafe { std::mem::transmute::<_, &'b mut MeshPass<'a>>(render_pass) }
	}
}

#[repr(transparent)]
pub struct MeshPass<'a>(wgpu::RenderPass<'a>);

#[derive(Debug)]
pub struct Mesh {
	pub buffer: wgpu::Buffer,
	pub instances: u32,
}

impl Mesh {
	pub fn new(state: &State, indices: &[u32]) -> Self {
		let buffer = state
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
		// property: &'a PointCloudProperty,
	) {
		mesh_pass
			.0
			.set_vertex_buffer(0, point_cloud.buffer.slice(..));
		// mesh_pass.0.set_vertex_buffer(
		// 	1,
		// 	property
		// 		.buffer
		// 		.slice(0..(point_cloud.instances * std::mem::size_of::<u32>() as u32) as wgpu::BufferAddress),
		// );
		mesh_pass
			.0
			.set_index_buffer(self.buffer.slice(..), wgpu::IndexFormat::Uint32);
		mesh_pass.0.draw_indexed(0..self.instances, 0, 0..1);
	}
}
