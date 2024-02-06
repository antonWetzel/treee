use common::MAX_LEAF_SIZE;
use math::Vector;
use wgpu::util::DeviceExt;

use crate::{depth_texture::DepthTexture, Camera3DGPU, Has, Lookup, Point, RenderPass, State};

pub struct PointCloudState {
	base: wgpu::Buffer,
	pipeline: wgpu::RenderPipeline,
}

//tan(60°)
const TAN_60_DEGREES: f32 = 1.732_050_8;

const BASE_VERTICES: [crate::PointEdge; 3] = [
	crate::PointEdge {
		position: Vector::new([-TAN_60_DEGREES, -1.0]),
	},
	crate::PointEdge {
		position: Vector::new([TAN_60_DEGREES, -1.0]),
	},
	crate::PointEdge { position: Vector::new([0.0, 2.0]) },
];

// Triangle is a lot faster then Square
// const BASE_VERTICES: [crate::PointEdge; 6] = [
// 	crate::PointEdge { position: Vector::new([-1.0, -1.0]) },
// 	crate::PointEdge { position: Vector::new([1.0, -1.0]) },
// 	crate::PointEdge { position: Vector::new([1.0, 1.0]) },
// 	crate::PointEdge { position: Vector::new([-1.0, -1.0]) },
// 	crate::PointEdge { position: Vector::new([1.0, 1.0]) },
// 	crate::PointEdge { position: Vector::new([-1.0, 1.0]) },
// ];

// Triangle is a lot faster then Octagon
//tan(22.5°)
// const TAN_225_DEGREES: f32 = 0.41421356237309503;

// const BASE_VERTICES: [crate::PointEdge; 18] = [
// 	crate::PointEdge {
// 		position: Vector::new([-TAN_225_DEGREES, -1.0]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([TAN_225_DEGREES, -1.0]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([-1.0, -TAN_225_DEGREES]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([-1.0, -TAN_225_DEGREES]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([TAN_225_DEGREES, -1.0]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([1.0, -TAN_225_DEGREES]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([-1.0, -TAN_225_DEGREES]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([1.0, -TAN_225_DEGREES]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([-1.0, TAN_225_DEGREES]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([-1.0, TAN_225_DEGREES]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([1.0, -TAN_225_DEGREES]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([1.0, TAN_225_DEGREES]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([-1.0, TAN_225_DEGREES]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([1.0, TAN_225_DEGREES]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([-TAN_225_DEGREES, 1.0]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([-TAN_225_DEGREES, 1.0]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([TAN_225_DEGREES, 1.0]),
// 	},
// 	crate::PointEdge {
// 		position: Vector::new([1.0, TAN_225_DEGREES]),
// 	},
// ];

impl PointCloudState {
	pub fn new(state: &impl Has<State>) -> Self {
		let state = state.get();

		let shader = state
			.device
			.create_shader_module(wgpu::include_wgsl!("point_cloud.wgsl"));
		let render_pipeline_layout = state
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
						Point::base_description(),
						Point::description(wgpu::VertexStepMode::Instance),
						Point::property_description(wgpu::VertexStepMode::Instance),
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
}

#[repr(transparent)]
pub struct PointCloudPass<'a>(wgpu::RenderPass<'a>);

pub trait PointCloudRender {
	fn render<'a>(&'a self, point_cloud_pass: &mut PointCloudPass<'a>);
}

pub trait PointCloudExt<'a, V, S> {
	fn render_point_clouds(
		&mut self,
		value: &'a V,
		state: &'a S,
		camera: &'a Camera3DGPU,
		lookup: &'a Lookup,
		environment: &'a PointCloudEnvironment,
	);
}

impl<'a, V, S> PointCloudExt<'a, V, S> for RenderPass<'a>
where
	S: Has<PointCloudState>,
	V: PointCloudRender,
{
	fn render_point_clouds(
		&mut self,
		value: &'a V,
		state: &'a S,
		camera: &'a Camera3DGPU,
		lookup: &'a Lookup,
		environment: &'a PointCloudEnvironment,
	) {
		self.set_pipeline(&state.get().pipeline);
		self.set_bind_group(0, camera.get_bind_group(), &[]);
		self.set_bind_group(1, &environment.bind_group, &[]);
		self.set_bind_group(2, lookup.get_bind_group(), &[]);
		self.set_vertex_buffer(0, state.get().base.slice(..));
		let lines_pass = unsafe { std::mem::transmute::<_, &mut PointCloudPass<'a>>(self) };
		value.render(lines_pass);
	}
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
				contents: bytemuck::cast_slice(vertices),
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
		point_cloud_pass
			.0
			.draw(0..BASE_VERTICES.len() as u32, 0..self.instances);
	}
}

pub struct PointCloudProperty {
	pub buffer: wgpu::Buffer,
	pub length: u32,
}

impl PointCloudProperty {
	pub fn new(state: &impl Has<State>, data: &[u32]) -> Self {
		let buffer = state
			.get()
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("point cloud property buffer"),
				contents: bytemuck::cast_slice(data),
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

pub struct PointCloudEnvironment {
	bind_group: wgpu::BindGroup,
	pub min: u32,
	pub max: u32,
	pub scale: f32,
}

impl PointCloudEnvironment {
	pub fn new(state: &impl Has<State>, min: u32, max: u32, scale: f32) -> Self {
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
			.get()
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("point cloud environment buffer"),
				contents: bytemuck::cast_slice(&[uniform]),
				usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			});

		let bind_group = state
			.get()
			.device
			.create_bind_group(&wgpu::BindGroupDescriptor {
				layout: &Self::get_layout(state),
				entries: &[wgpu::BindGroupEntry {
					binding: 0,
					resource: buffer.as_entire_binding(),
				}],
				label: Some("point cloud environment bindgroup"),
			});
		Self { bind_group, min, max, scale }
	}

	pub fn get_layout(state: &impl Has<State>) -> wgpu::BindGroupLayout {
		state
			.get()
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
