use nalgebra as na;
use wgpu::{util::DeviceExt, vertex_attr_array};

use crate::{depth_texture::DepthTexture, Camera3DGPU, PointCloud, RenderPass, State};

pub struct LinesState {
	pipeline: wgpu::RenderPipeline,
}

impl LinesState {
	pub fn new(state: &State) -> Self {
		let shader = state
			.device
			.create_shader_module(wgpu::include_wgsl!("lines.wgsl"));
		let render_pipeline_layout = state
			.device
			.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("Lines Pipeline Layout"),
				bind_group_layouts: &[&Camera3DGPU::get_layout(state)],
				push_constant_ranges: &[],
			});

		let pipeline = state
			.device
			.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
				label: Some("lines"),
				layout: Some(&render_pipeline_layout),
				vertex: wgpu::VertexState {
					module: &shader,
					entry_point: "vs_main",
					buffers: &[description(wgpu::VertexStepMode::Vertex)],
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
					topology: wgpu::PrimitiveTopology::LineList,
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

	pub fn render<'a, 'b>(
		&'a self,
		render_pass: &'b mut RenderPass<'a>,
		camera: &'a Camera3DGPU,
	) -> &'b mut LinesPass<'a> {
		render_pass.set_pipeline(&self.pipeline);
		render_pass.set_bind_group(0, camera.get_bind_group(), &[]);
		unsafe { std::mem::transmute::<_, &mut LinesPass<'a>>(render_pass) }
	}
}

#[repr(transparent)]
pub struct LinesPass<'a>(wgpu::RenderPass<'a>);

#[derive(Debug)]
pub struct Lines {
	pub indices: wgpu::Buffer,
	pub instances: u32,
}

impl Lines {
	pub fn new(state: &State, indices: &[u32]) -> Self {
		let indices_buffer = state
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("lines indices buffer"),
				contents: bytemuck::cast_slice(indices),
				usage: wgpu::BufferUsages::INDEX,
			});

		Self {
			indices: indices_buffer,
			instances: indices.len() as u32,
		}
	}

	pub fn render<'a>(&'a self, points: &'a PointCloud, lines_pass: &mut LinesPass<'a>) {
		lines_pass.0.set_vertex_buffer(0, points.buffer.slice(..));
		lines_pass
			.0
			.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
		lines_pass.0.draw_indexed(0..self.instances, 0, 0..1);
	}
}

const ATTRIBUTES: [wgpu::VertexAttribute; 1] = vertex_attr_array![0 => Float32x3];

pub fn description<'a>(step_mode: wgpu::VertexStepMode) -> wgpu::VertexBufferLayout<'a> {
	wgpu::VertexBufferLayout {
		array_stride: std::mem::size_of::<na::Point3<f32>>() as wgpu::BufferAddress,
		step_mode,
		attributes: &ATTRIBUTES,
	}
}
