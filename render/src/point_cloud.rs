use nalgebra as na;
use wgpu::util::DeviceExt;

use crate::{
	depth_texture::DepthTexture, point_base_description, point_description,
	point_property_description, Camera3DGPU, Lookup, PointEdge, RenderPass, State,
};

#[derive(Debug)]
pub struct PointCloudState {
	base: wgpu::Buffer,
	pipeline: wgpu::RenderPipeline,
}

//tan(60°)
const DIFF: f32 = 1.732_050_8;

pub const BASE_VERTICES: [PointEdge; 3] = [
	PointEdge { position: na::Point2::new(-DIFF, -1.0) },
	PointEdge { position: na::Point2::new(DIFF, -1.0) },
	PointEdge { position: na::Point2::new(0.0, 2.0) },
];

impl PointCloudState {
	pub fn new(state: &State) -> Self {
		let shader = state
			.device
			.create_shader_module(wgpu::include_wgsl!("point_cloud.wgsl"));
		let render_pipeline_layout =
			state
				.device
				.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
					label: Some("Render Pipeline Layout"),
					bind_group_layouts: &[
						&Camera3DGPU::get_layout(state),
						&PointCloudEnvironment::get_layout(state),
						&Lookup::get_layout(state),
					],
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
						point_base_description(),
						point_description(wgpu::VertexStepMode::Instance),
						point_property_description(wgpu::VertexStepMode::Instance),
					],
					compilation_options: Default::default(),
				},
				fragment: Some(wgpu::FragmentState {
					module: &shader,
					entry_point: "fs_main",
					targets: &[Some(wgpu::ColorTargetState {
						format: state.surface_format,
						blend: Some(wgpu::BlendState::REPLACE),
						write_mask: wgpu::ColorWrites::ALL,
					})],
					compilation_options: Default::default(),
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
				cache: None,
			});

		Self {
			base: state
				.device
				.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("Quad Buffer"),
					contents: bytemuck::cast_slice(&BASE_VERTICES),
					usage: wgpu::BufferUsages::VERTEX,
				}),
			pipeline,
		}
	}

	pub fn render<'a, 'b>(
		&'a self,
		render_pass: &'b mut RenderPass<'a>,
		camera: &'a Camera3DGPU,
		lookup: &'a Lookup,
		environment: &'a PointCloudEnvironment,
	) -> &'b mut PointCloudPass<'a> {
		render_pass.set_pipeline(&self.pipeline);
		render_pass.set_bind_group(0, camera.get_bind_group(), &[]);
		render_pass.set_bind_group(1, &environment.bind_group, &[]);
		render_pass.set_bind_group(2, lookup.get_bind_group(), &[]);
		render_pass.set_vertex_buffer(0, self.base.slice(..));
		unsafe { std::mem::transmute::<_, &mut PointCloudPass<'a>>(render_pass) }
	}
}

#[repr(transparent)]
pub struct PointCloudPass<'a>(wgpu::RenderPass<'a>);

impl<'a> PointCloudPass<'a> {
	pub fn lookup(&mut self, lookup: &'a Lookup) {
		self.0.set_bind_group(2, lookup.get_bind_group(), &[]);
	}
}

#[derive(Debug)]
pub struct PointCloud {
	pub buffer: wgpu::Buffer,
	pub instances: u32,
}

impl PointCloud {
	pub fn new(state: &State, vertices: &[na::Point3<f32>]) -> Self {
		let buffer = state
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("point cloud buffer"),
				contents: bytemuck::cast_slice(vertices),
				usage: wgpu::BufferUsages::VERTEX,
			});

		Self { buffer, instances: vertices.len() as u32 }
	}

	pub fn render<'a>(
		&'a self,
		point_cloud_pass: &mut PointCloudPass<'a>,
		property: &'a PointCloudProperty,
	) {
		point_cloud_pass
			.0
			.set_vertex_buffer(1, self.buffer.slice(..));
		point_cloud_pass.0.set_vertex_buffer(
			2,
			property.buffer.slice(
				0..(self.instances * std::mem::size_of::<u32>() as u32) as wgpu::BufferAddress,
			),
		);
		if property.length != 0 {
			assert!(
				property.length == self.instances,
				"{} {}",
				property.length,
				self.instances
			);
		}
		point_cloud_pass
			.0
			.draw(0..BASE_VERTICES.len() as u32, 0..self.instances);
	}
}

#[derive(Debug)]
pub struct PointCloudProperty {
	pub buffer: wgpu::Buffer,
	pub length: u32,
}

impl PointCloudProperty {
	pub fn new(state: &State, data: &[u32]) -> Self {
		let buffer = state
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("point cloud property buffer"),
				contents: bytemuck::cast_slice(data),
				usage: wgpu::BufferUsages::VERTEX,
			});

		Self { buffer, length: data.len() as u32 }
	}

	pub fn new_empty(state: &State, size: usize) -> Self {
		let buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("point cloud property buffer"),
			size: (size * std::mem::size_of::<u32>()) as u64,
			usage: wgpu::BufferUsages::VERTEX,
			mapped_at_creation: false,
		});
		Self { buffer, length: 0 }
	}
}

pub struct PointCloudEnvironment {
	bind_group: wgpu::BindGroup,
	pub min: u32,
	pub max: u32,
	pub scale: f32,
}

impl PointCloudEnvironment {
	pub fn new(state: &State, min: u32, max: u32, scale: f32) -> Self {
		#[repr(C)]
		#[derive(Debug, Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
		struct Uniform {
			scale: f32,
			min: u32,
			max: u32,
			pad: [u32; 2],
		}

		let uniform = Uniform { scale, min, max, pad: [0, 0] };
		let buffer = state
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("point cloud environment buffer"),
				contents: bytemuck::cast_slice(&[uniform]),
				usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			});

		let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &Self::get_layout(state),
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: buffer.as_entire_binding(),
			}],
			label: Some("point cloud environment bindgroup"),
		});
		Self { bind_group, min, max, scale }
	}

	pub fn update(&mut self, state: &State) {
		*self = Self::new(state, self.min, self.max, self.scale);
	}

	pub fn get_layout(state: &State) -> wgpu::BindGroupLayout {
		state
			.device
			.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
				entries: &[wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStages::VERTEX,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				}],
				label: Some("point cloud environment layout"),
			})
	}
}
