use nalgebra as na;
use wgpu::util::DeviceExt;

use crate::{depth_texture::DepthTexture, RenderPass, State, Vertex2D};

const FULL_SCREEN_VERTICES: [Vertex2D; 3] = [
	Vertex2D {
		position: [-1.0, -1.0],
		tex_coords: [0.0, 1.0],
	},
	Vertex2D {
		position: [3.0, -1.0],
		tex_coords: [2.0, 1.0],
	},
	Vertex2D {
		position: [-1.0, 3.0],
		tex_coords: [0.0, -1.0],
	},
];

pub struct EyeDome {
	depth_layout: wgpu::BindGroupLayout,
	depth_bind_group: wgpu::BindGroup,

	settings_layout: wgpu::BindGroupLayout,
	settings_bind_group: wgpu::BindGroup,

	vertex_buffer: wgpu::Buffer,
	render_pipeline: wgpu::RenderPipeline,

	pub color: na::Point3<f32>,
	pub strength: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct EyeDomeUniform {
	color: [f32; 3],
	strength: f32,
}

impl EyeDome {
	pub fn new(
		state: &State,
		config: &wgpu::SurfaceConfiguration,
		depth: &DepthTexture,
		strength: f32,
	) -> Self {
		let depth_layout =
			state
				.device
				.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
					label: Some("eye dome Layout"),
					entries: &[wgpu::BindGroupLayoutEntry {
						binding: 0,
						count: None,
						ty: wgpu::BindingType::Texture {
							sample_type: wgpu::TextureSampleType::Float { filterable: false },
							multisampled: false,
							view_dimension: wgpu::TextureViewDimension::D2,
						},
						visibility: wgpu::ShaderStages::FRAGMENT,
					}],
				});

		let depth_bind_group = Self::get_depth_bindgroup(state, &depth_layout, depth);

		let settings_layout =
			state
				.device
				.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
					label: Some("eye dome Layout"),
					entries: &[wgpu::BindGroupLayoutEntry {
						binding: 0,
						count: None,
						ty: wgpu::BindingType::Buffer {
							ty: wgpu::BufferBindingType::Uniform,
							has_dynamic_offset: false,
							min_binding_size: None,
						},
						visibility: wgpu::ShaderStages::FRAGMENT,
					}],
				});

		let color = [0.0, 0.0, 0.0].into();

		let settings_bind_group =
			Self::get_settings_bindgroup(state, &settings_layout, color, strength);

		let vertex_buffer = state
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("eye dome vertex buffer"),
				contents: bytemuck::cast_slice(&FULL_SCREEN_VERTICES),
				usage: wgpu::BufferUsages::VERTEX,
			});

		let pipeline_layout =
			state
				.device
				.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
					label: Some("eye dome Pipeline Layout"),
					bind_group_layouts: &[&depth_layout, &settings_layout],
					push_constant_ranges: &[],
				});

		let shader = state
			.device
			.create_shader_module(wgpu::ShaderModuleDescriptor {
				label: Some("eye dome Display Shader"),
				source: wgpu::ShaderSource::Wgsl(include_str!("eye_dome.wgsl").into()),
			});

		let render_pipeline =
			state
				.device
				.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
					label: Some("eye dome Render Pipeline"),
					layout: Some(&pipeline_layout),
					vertex: wgpu::VertexState {
						module: &shader,
						entry_point: "vs_main",
						buffers: &[Vertex2D::desc()],
						compilation_options: Default::default(),
					},
					fragment: Some(wgpu::FragmentState {
						module: &shader,
						entry_point: "fs_main",
						targets: &[Some(wgpu::ColorTargetState {
							format: config.format,
							blend: Some(wgpu::BlendState::ALPHA_BLENDING),
							write_mask: wgpu::ColorWrites::ALL,
						})],
						compilation_options: Default::default(),
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
					cache: None,
				});

		Self {
			depth_layout,
			depth_bind_group,

			settings_layout,
			settings_bind_group,

			vertex_buffer,
			render_pipeline,

			color,
			strength,
		}
	}

	pub fn update_depth(&mut self, state: &State, depth: &DepthTexture) {
		self.depth_bind_group = Self::get_depth_bindgroup(state, &self.depth_layout, depth);
	}

	fn get_depth_bindgroup(
		state: &State,
		layout: &wgpu::BindGroupLayout,
		depth: &DepthTexture,
	) -> wgpu::BindGroup {
		state.device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout,
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: wgpu::BindingResource::TextureView(&depth.view),
			}],
			label: Some("eye dome bind group"),
		})
	}

	pub fn update_settings(&mut self, state: &State) {
		self.settings_bind_group =
			Self::get_settings_bindgroup(state, &self.settings_layout, self.color, self.strength);
	}

	fn get_settings_bindgroup(
		state: &State,
		layout: &wgpu::BindGroupLayout,
		color: na::Point3<f32>,
		strength: f32,
	) -> wgpu::BindGroup {
		let strength = 1.0 - strength;
		let uniform = EyeDomeUniform {
			color: color.coords.data.0[0],
			strength: if strength < 0.1 { 0.1 } else { strength }.powi(6),
		};
		let buffer = state
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Camera Buffer"),
				contents: bytemuck::cast_slice(&[uniform]),
				usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			});

		state.device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout,
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: buffer.as_entire_binding(),
			}],
			label: Some("eye dome bind group"),
		})
	}

	pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
		render_pass.set_pipeline(&self.render_pipeline);
		render_pass.set_bind_group(0, &self.depth_bind_group, &[]);
		render_pass.set_bind_group(1, &self.settings_bind_group, &[]);
		render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
		render_pass.draw(0..(FULL_SCREEN_VERTICES.len() as u32), 0..1);
	}
}
