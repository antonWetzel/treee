use common::MAX_LEAF_SIZE;
use math::Vector;
use wgpu::util::DeviceExt;

use crate::{depth_texture::DepthTexture, Camera3DGPU, Has, Lookup, Point, RenderPass, State};

pub struct PointCloudState {
	quad: wgpu::Buffer,
	pipeline: wgpu::RenderPipeline,
}

impl PointCloudState {
	pub fn new(state: &impl Has<State>) -> Self {
		let state = state.get();
		const QUAD_DATA: [crate::PointEdge; 6] = [
			crate::PointEdge { position: Vector::new([-1.0, -1.0]) },
			crate::PointEdge { position: Vector::new([1.0, -1.0]) },
			crate::PointEdge { position: Vector::new([1.0, 1.0]) },
			crate::PointEdge { position: Vector::new([-1.0, -1.0]) },
			crate::PointEdge { position: Vector::new([1.0, 1.0]) },
			crate::PointEdge { position: Vector::new([-1.0, 1.0]) },
		];

		let shader = state
			.device
			.create_shader_module(wgpu::include_wgsl!("point_cloud.wgsl"));
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
				label: Some("pointcloud"),
				layout: Some(&render_pipeline_layout),
				vertex: wgpu::VertexState {
					module: &shader,
					entry_point: "vs_main",
					buffers: &[
						Point::quad_description(),
						Point::description(),
						Point::property_description(),
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

		Self {
			quad: state
				.device
				.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("Quad Buffer"),
					contents: bytemuck::cast_slice(&QUAD_DATA),
					usage: wgpu::BufferUsages::VERTEX,
				}),
			pipeline,
		}
	}
}

pub struct PointCloudPass<'a>(wgpu::RenderPass<'a>);

impl<'a> PointCloudPass<'a> {
	pub fn start(
		mut render_pass: RenderPass<'a>,
		state: &'a impl Has<PointCloudState>,
		environment: &'a impl PointCloudEnvironment,
	) -> Self {
		let state = state.get();
		render_pass.set_pipeline(&state.pipeline);
		environment.camera().bind(&mut render_pass, 0);
		render_pass.set_vertex_buffer(0, state.get().quad.slice(..));
		render_pass.set_bind_group(1, environment.lookup().get_bind_group(), &[]);
		Self(render_pass)
	}

	pub fn end(self) -> RenderPass<'a> {
		self.0
	}
}

pub trait PointCloudEnvironment {
	fn camera(&self) -> &Camera3DGPU;
	fn lookup(&self) -> &Lookup;
}

#[derive(Debug)]
pub struct PointCloud {
	pub buffer: wgpu::Buffer,
	pub instances: u32,
}

impl PointCloud {
	pub fn new(state: &impl Has<State>, vertices: &[crate::Point]) -> Self {
		let buffer = state
			.get()
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("point cloud buffer"),
				contents: bytemuck::cast_slice(&vertices[..]),
				usage: wgpu::BufferUsages::VERTEX,
			});

		Self { buffer, instances: vertices.len() as u32 }
	}

	pub fn render<'a>(&'a self, point_cloud_pass: &mut PointCloudPass<'a>, property: &'a PointCloudProperty) {
		point_cloud_pass
			.0
			.set_vertex_buffer(1, self.buffer.slice(..));
		point_cloud_pass.0.set_vertex_buffer(
			2,
			property
				.buffer
				.slice(0..(self.instances * std::mem::size_of::<u32>() as u32) as wgpu::BufferAddress),
		);
		if property.length != 0 {
			assert!(
				property.length == self.instances,
				"{} {}",
				property.length,
				self.instances
			);
		}
		point_cloud_pass.0.draw(0..6, 0..self.instances);
	}
}

pub struct PointCloudProperty {
	pub buffer: wgpu::Buffer,
	length: u32,
}

impl PointCloudProperty {
	pub fn new(state: &impl Has<State>, data: &[u32]) -> Self {
		let buffer = state
			.get()
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("point cloud property buffer"),
				contents: bytemuck::cast_slice(&data[..]),
				usage: wgpu::BufferUsages::VERTEX,
			});

		Self { buffer, length: data.len() as u32 }
	}

	pub fn new_empty(state: &impl Has<State>) -> Self {
		let state: &State = state.get();
		let buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("point cloud property buffer"),
			size: (MAX_LEAF_SIZE * std::mem::size_of::<u32>()) as u64,
			usage: wgpu::BufferUsages::VERTEX,
			mapped_at_creation: false,
		});
		Self { buffer, length: 0 }
	}
}
