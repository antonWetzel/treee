use math::Vector;
use wgpu::util::DeviceExt;

use crate::{depth_texture::DepthTexture, Camera3DGPU, Has, Lookup, Point, RenderPass, State};

pub struct PointCloudState {
	quad: wgpu::Buffer,
	pipeline: wgpu::RenderPipeline,
}

impl PointCloudState {
	pub fn new(state: &impl Has<State>) -> Self {
		const QUAD_DATA: [crate::PointEdge; 6] = [
			crate::PointEdge { position: Vector::new([-1.0, -1.0]) },
			crate::PointEdge { position: Vector::new([1.0, -1.0]) },
			crate::PointEdge { position: Vector::new([1.0, 1.0]) },
			crate::PointEdge { position: Vector::new([-1.0, -1.0]) },
			crate::PointEdge { position: Vector::new([1.0, 1.0]) },
			crate::PointEdge { position: Vector::new([-1.0, 1.0]) },
		];
		Self {
			quad: state
				.get()
				.device
				.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("Quad Buffer"),
					contents: bytemuck::cast_slice(&QUAD_DATA),
					usage: wgpu::BufferUsages::VERTEX,
				}),
			pipeline: create_pipeline(
				state.get(),
				wgpu::include_wgsl!("point_cloud.wgsl"),
				&[Point::quad_description(), Point::description()],
				&[&Camera3DGPU::get_layout(state), &Lookup::get_layout(state)],
				Some("pointcloud"),
				true,
			),
		}
	}

	pub fn activate<'a>(&'a self, mut render_pass: RenderPass<'a>, lookup: &'a Lookup) -> PointCloudPass<'a> {
		render_pass.set_vertex_buffer(0, self.quad.slice(..));
		render_pass.set_bind_group(1, lookup.get_bind_group(), &[]);
		PointCloudPass(render_pass)
	}
}

#[repr(transparent)]
pub struct PointCloudPass<'a>(wgpu::RenderPass<'a>);

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
		point_cloud_pass
			.0
			.set_vertex_buffer(1, self.buffer.slice(..));
		point_cloud_pass.0.draw(0..6, 0..self.instances);
	}
}

pub trait PointCloudStateExtension
where
	Self: Sized,
{
	fn render_point_clouds<'a>(
		&'a self,
		render_pass: RenderPass<'a>,
		renderable: &'a impl RenderablePointCloud<Self>,
	) -> RenderPass<'a>;
}

impl<S> PointCloudStateExtension for S
where
	S: Has<PointCloudState>,
{
	fn render_point_clouds<'a>(
		&'a self,
		mut render_pass: RenderPass<'a>,
		renderable: &'a impl RenderablePointCloud<S>,
	) -> RenderPass<'a> {
		let state = self.get();
		render_pass.set_pipeline(&state.pipeline);
		renderable.get_cam().bind(&mut render_pass, 0);
		render_pass.set_vertex_buffer(0, state.get().quad.slice(..));
		render_pass.set_bind_group(1, renderable.get_lookup().get_bind_group(), &[]);
		renderable.render(PointCloudPass(render_pass), self).0
	}
}

pub trait RenderablePointCloud<State> {
	fn get_cam(&self) -> &Camera3DGPU;
	fn get_lookup(&self) -> &Lookup;
	fn render<'a>(&'a self, render_pass: PointCloudPass<'a>, state: &'a State) -> PointCloudPass<'a>;
}

fn create_pipeline(
	state: &State,
	wgsl: wgpu::ShaderModuleDescriptor,
	vertex_descriptions: &[wgpu::VertexBufferLayout],
	bind_group_layouts: &[&wgpu::BindGroupLayout],
	label: Option<&str>,
	depth: bool,
) -> wgpu::RenderPipeline {
	let shader = state.device.create_shader_module(wgsl);
	let render_pipeline_layout = state
		.device
		.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Render Pipeline Layout"),
			bind_group_layouts,
			push_constant_ranges: &[],
		});

	state
		.device
		.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label,
			layout: Some(&render_pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: "vs_main",
				buffers: vertex_descriptions,
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
			depth_stencil: if depth {
				Some(wgpu::DepthStencilState {
					format: DepthTexture::DEPTH_FORMAT,
					depth_write_enabled: true,
					depth_compare: wgpu::CompareFunction::Less,
					stencil: wgpu::StencilState::default(),
					bias: wgpu::DepthBiasState::default(),
				})
			} else {
				None
			},
			multisample: wgpu::MultisampleState {
				count: 1,
				mask: !0,
				alpha_to_coverage_enabled: false,
			},
			multiview: None,
		})
}
