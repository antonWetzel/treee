use wgpu::util::DeviceExt;

use crate::{depth_texture::DepthTexture, RenderPass, State};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
	position: [f32; 2],
	tex_coords: [f32; 2],
}

impl Vertex {
	const ATTRIBUTES: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

	fn desc() -> wgpu::VertexBufferLayout<'static> {
		use std::mem;
		wgpu::VertexBufferLayout {
			array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: Self::ATTRIBUTES,
		}
	}
}

const FULL_SCREEN_VERTICES: &[Vertex] = &[
	Vertex {
		position: [-1.0, -1.0],
		tex_coords: [0.0, 1.0],
	},
	Vertex {
		position: [3.0, -1.0],
		tex_coords: [2.0, 1.0],
	},
	Vertex {
		position: [-1.0, 3.0],
		tex_coords: [0.0, -1.0],
	},
];

pub struct EyeDome {
	layout: wgpu::BindGroupLayout,
	bind_group: wgpu::BindGroup,
	vertex_buffer: wgpu::Buffer,
	render_pipeline: wgpu::RenderPipeline,
}

impl EyeDome {
	pub fn new(
		state: &State,
		config: &wgpu::SurfaceConfiguration,
		depth: &DepthTexture,
		source: &wgpu::Texture,
	) -> Self {
		let layout = state
			.device
			.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
				label: Some("eye dome Layout"),
				entries: &[
					wgpu::BindGroupLayoutEntry {
						binding: 0,
						count: None,
						ty: wgpu::BindingType::Texture {
							sample_type: wgpu::TextureSampleType::Float { filterable: false },
							multisampled: false,
							view_dimension: wgpu::TextureViewDimension::D2,
						},
						visibility: wgpu::ShaderStages::FRAGMENT,
					},
					wgpu::BindGroupLayoutEntry {
						binding: 1,
						count: None,
						ty: wgpu::BindingType::Texture {
							sample_type: wgpu::TextureSampleType::Float { filterable: false },
							multisampled: false,
							view_dimension: wgpu::TextureViewDimension::D2,
						},
						visibility: wgpu::ShaderStages::FRAGMENT,
					},
				],
			});

		let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&depth.view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(
						&source.create_view(&wgpu::TextureViewDescriptor::default()),
					),
				},
			],
			label: Some("eye dome bind group"),
		});

		let vertex_buffer = state
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("eye dome vertex buffer"),
				contents: bytemuck::cast_slice(FULL_SCREEN_VERTICES),
				usage: wgpu::BufferUsages::VERTEX,
			});

		let pipeline_layout = state
			.device
			.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("eye dome Pipeline Layout"),
				bind_group_layouts: &[&layout],
				push_constant_ranges: &[],
			});

		let shader = state
			.device
			.create_shader_module(wgpu::ShaderModuleDescriptor {
				label: Some("eye dome Display Shader"),
				source: wgpu::ShaderSource::Wgsl(include_str!("../assets/eye_dome.wgsl").into()),
			});

		let render_pipeline = state
			.device
			.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
				label: Some("eye dome Render Pipeline"),
				layout: Some(&pipeline_layout),
				vertex: wgpu::VertexState {
					module: &shader,
					entry_point: "vs_main",
					buffers: &[Vertex::desc()],
				},
				fragment: Some(wgpu::FragmentState {
					module: &shader,
					entry_point: "fs_main",
					targets: &[Some(wgpu::ColorTargetState {
						format: config.format,
						blend: Some(wgpu::BlendState {
							color: wgpu::BlendComponent::REPLACE,
							alpha: wgpu::BlendComponent::REPLACE,
						}),
						write_mask: wgpu::ColorWrites::ALL,
					})],
				}),
				primitive: wgpu::PrimitiveState {
					topology: wgpu::PrimitiveTopology::TriangleList,
					strip_index_format: None,
					front_face: wgpu::FrontFace::Ccw,
					cull_mode: Some(wgpu::Face::Back),
					polygon_mode: wgpu::PolygonMode::Fill,
					unclipped_depth: false,
					conservative: false,
				},
				depth_stencil: None,
				multisample: wgpu::MultisampleState {
					count: 1,
					mask: !0,
					alpha_to_coverage_enabled: false,
				},
				multiview: None,
			});

		Self {
			layout,
			bind_group,
			vertex_buffer,
			render_pipeline,
		}
	}

	pub fn update(&mut self, state: &State, depth: &DepthTexture, source: &wgpu::Texture) {
		self.bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &self.layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&depth.view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(
						&source.create_view(&wgpu::TextureViewDescriptor::default()),
					),
				},
			],
			label: Some("eye dome bind group"),
		});
	}

	pub fn render<'a>(&'a self, mut render_pass: RenderPass<'a>) -> RenderPass<'a> {
		render_pass.set_pipeline(&self.render_pipeline);
		render_pass.set_bind_group(0, &self.bind_group, &[]);
		render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
		render_pass.draw(0..(FULL_SCREEN_VERTICES.len() as u32), 0..1);
		render_pass
	}
}
