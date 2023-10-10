use crate::{depth_texture::DepthTexture, Camera3DGPU, Has, Point, Renderable, State};

pub struct Pipeline3D {
	pipeline: wgpu::RenderPipeline,
}

impl Pipeline3D {
	pub fn new(state: &impl Has<State>) -> Self {
		return Self {
			pipeline: create_pipeline(
				state,
				wgpu::include_wgsl!("pipeline_3d.wgsl"),
				&[Point::quad_description(), Point::description()],
				&[&Camera3DGPU::get_layout(state)],
				Some("3d"),
				true,
			),
		};
	}

	pub fn render<'a, S>(
		&'a self,
		mut render_pass: wgpu::RenderPass<'a>,
		cam: &'a Camera3DGPU,
		renderable: &'a impl Renderable<S>,
		state: &'static S,
	) -> wgpu::RenderPass<'a> {
		render_pass.set_pipeline(&self.pipeline);
		cam.bind(&mut render_pass, 0);
		renderable.render(render_pass, state)
	}
}

fn create_pipeline(
	state: &impl Has<State>,
	wgsl: wgpu::ShaderModuleDescriptor,
	vertex_descriptions: &[wgpu::VertexBufferLayout],
	bind_group_layouts: &[&wgpu::BindGroupLayout],
	label: Option<&str>,
	depth: bool,
) -> wgpu::RenderPipeline {
	let state = state.get();
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
