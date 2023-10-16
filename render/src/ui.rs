use math::{Transform, Vector, X, Y};
use wgpu::{util::DeviceExt, SurfaceConfiguration};
use wgpu_text::{
	glyph_brush::{ab_glyph::FontRef, Section, Text},
	BrushBuilder, TextBrush,
};

pub struct UIState {
	pipeline: wgpu::RenderPipeline,
	sampler: wgpu::Sampler,
	height: f32,
}

impl UIState {
	pub fn new(state: &impl Has<State>, height: f32) -> Self {
		let state: &State = state.get();
		let shader = state
			.device
			.create_shader_module(wgpu::include_wgsl!("ui.wgsl"));
		let render_pipeline_layout = state
			.device
			.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("Render Pipeline Layout"),
				bind_group_layouts: &[
					&UI::get_projection_layout(state),
					&UIImage::get_layout(state),
				],
				push_constant_ranges: &[],
			});

		let pipeline = state
			.device
			.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
				label: Some("ui pipeline"),
				layout: Some(&render_pipeline_layout),
				vertex: wgpu::VertexState {
					module: &shader,
					entry_point: "vs_main",
					buffers: &[Vertex2D::desc()],
				},
				fragment: Some(wgpu::FragmentState {
					module: &shader,
					entry_point: "fs_main",
					targets: &[Some(wgpu::ColorTargetState {
						format: state.surface_format,
						blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
				depth_stencil: None,
				multisample: wgpu::MultisampleState {
					count: 1,
					mask: !0,
					alpha_to_coverage_enabled: false,
				},
				multiview: None,
			});

		let sampler = state.device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Nearest,
			mipmap_filter: wgpu::FilterMode::Nearest,
			..Default::default()
		});

		Self { sampler, pipeline, height }
	}
}

use crate::{Has, RenderPass, State, Texture, Vertex2D};

pub struct UI {
	brush: TextBrush<FontRef<'static>>,
	projection: wgpu::BindGroup,
	scale: f32,
}

impl UI {
	pub fn new(state: &(impl Has<State> + Has<UIState>), config: &SurfaceConfiguration) -> Self {
		let (render_state, ui_state): (&State, &UIState) = (state.get(), state.get());
		let font = include_bytes!("ui_Urbanist-Bold.ttf");
		let font = FontRef::try_from_slice(font).unwrap();

		let brush = BrushBuilder::using_font(font).build(
			&render_state.device,
			config.width,
			config.height,
			config.format,
		);

		Self {
			brush,
			scale: config.height as f32 / ui_state.height,
			projection: Self::create_projection(state, config),
		}
	}

	pub fn resize(&mut self, state: &(impl Has<State> + Has<UIState>), config: &SurfaceConfiguration) {
		let (render_state, ui_state): (&State, &UIState) = (state.get(), state.get());
		self.brush.resize_view(
			config.width as f32,
			config.height as f32,
			&render_state.queue,
		);
		self.scale = config.height as f32 / ui_state.height;
		self.projection = Self::create_projection(state, config);
	}

	pub fn get_scale(&self) -> f32 {
		self.scale
	}

	pub fn queue(&mut self, state: &impl Has<State>, target: &impl UICollect) {
		let state = state.get();
		let mut collector = UICollector { data: Vec::new() };
		target.collect(&mut collector);
		self.brush
			.queue(&state.device, &state.queue, collector.data)
			.unwrap();
	}

	pub fn render<'a, S: Has<UIState>>(
		&'a self,
		renderable: &'a impl RenderableUI<S>,
		state: &'a S,
		mut render_pass: RenderPass<'a>,
	) -> RenderPass<'a> {
		let ui_state = state.get();
		self.brush.draw(&mut render_pass);
		render_pass.set_pipeline(&ui_state.pipeline);
		render_pass.set_bind_group(0, &self.projection, &[]);
		renderable.render(UIPass(render_pass), state).0
	}

	fn create_projection(state: &(impl Has<State> + Has<UIState>), config: &SurfaceConfiguration) -> wgpu::BindGroup {
		let (state, ui_state): (&State, &UIState) = (state.get(), state.get());
		let projection: Transform<2, f32> = Transform::translation([-1.0, 1.0].into())
			* Transform::scale(
				[
					1.0 * (config.height as f32 / config.width as f32) / ui_state.height * 2.0,
					-1.0 / ui_state.height * 2.0,
				]
				.into(),
			);
		let projection_buffer = state
			.get()
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("ui projection buffer"),
				contents: bytemuck::cast_slice(&[projection.as_matrix().raw()]),
				usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			});

		state
			.get()
			.device
			.create_bind_group(&wgpu::BindGroupDescriptor {
				layout: &Self::get_projection_layout(state),
				entries: &[wgpu::BindGroupEntry {
					binding: 0,
					resource: projection_buffer.as_entire_binding(),
				}],
				label: Some("projection bind group"),
			})
	}

	pub fn get_projection_layout(state: &impl Has<State>) -> wgpu::BindGroupLayout {
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
				label: Some("lookup layout"),
			})
	}
}

pub trait UICollect {
	fn collect<'a>(&'a self, collector: &mut UICollector<'a>);
}

pub struct UICollector<'a> {
	data: Vec<Section<'a>>,
}

impl<'a> UICollector<'a> {
	pub fn add_element(&mut self, element: &'a UIElement) {
		self.data.push(Section {
			screen_position: (element.position[X], element.position[Y]),
			text: element
				.text
				.iter()
				.map(|t| {
					Text::new(t.as_str())
						.with_scale(element.font_size)
						.with_color([0.0, 0.0, 0.0, 1.0])
				})
				.collect(),
			..Default::default()
		});
	}
}

pub struct UIElement {
	pub position: Vector<2, f32>,
	pub text: Vec<String>,
	pub font_size: f32,
}

impl UIElement {
	pub fn new(text: Vec<String>, position: Vector<2, f32>, font_size: f32) -> Self {
		Self { text, position, font_size }
	}
}

pub struct UIImage {
	bind_group: wgpu::BindGroup,
	buffer: wgpu::Buffer,
}

impl UIImage {
	pub fn new(
		state: &(impl Has<State> + Has<UIState>),
		texture: &Texture,
		position: Vector<2, f32>,
		size: Vector<2, f32>,
	) -> Self {
		let (state, ui_state): (&State, &UIState) = (state.get(), state.get());

		assert_eq!(
			texture.size[X] / size[X] as u32,
			texture.size[Y] / size[Y] as u32
		);

		let vertices = [
			Vertex2D {
				position: position.data(),
				tex_coords: [0.0, 0.0],
			},
			Vertex2D {
				position: (position + [size[X], size[Y]].into()).data(),
				tex_coords: [1.0, 1.0],
			},
			Vertex2D {
				position: (position + [size[X], 0.0].into()).data(),
				tex_coords: [1.0, 0.0],
			},
			Vertex2D {
				position: position.data(),
				tex_coords: [0.0, 0.0],
			},
			Vertex2D {
				position: (position + [0.0, size[Y]].into()).data(),
				tex_coords: [0.0, 1.0],
			},
			Vertex2D {
				position: (position + [size[X], size[Y]].into()).data(),
				tex_coords: [1.0, 1.0],
			},
		];

		let buffer = state
			.get()
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Vertex Buffer"),
				contents: bytemuck::cast_slice(&vertices[..]),
				usage: wgpu::BufferUsages::VERTEX,
			});

		let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &Self::get_layout(state),
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(
						&texture
							.gpu
							.create_view(&wgpu::TextureViewDescriptor::default()),
					),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(&ui_state.sampler),
				},
			],
			label: Some("diffuse_bind_group"),
		});

		Self { buffer, bind_group }
	}

	pub fn get_layout(state: &impl Has<State>) -> wgpu::BindGroupLayout {
		state
			.get()
			.device
			.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
				entries: &[
					wgpu::BindGroupLayoutEntry {
						binding: 0,
						visibility: wgpu::ShaderStages::FRAGMENT,
						ty: wgpu::BindingType::Texture {
							multisampled: false,
							view_dimension: wgpu::TextureViewDimension::D2,
							sample_type: wgpu::TextureSampleType::Float { filterable: true },
						},
						count: None,
					},
					wgpu::BindGroupLayoutEntry {
						binding: 1,
						visibility: wgpu::ShaderStages::FRAGMENT,
						ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
						count: None,
					},
				],
				label: Some("lookup layout"),
			})
	}

	pub fn render<'a>(&'a self, render_pass: &mut UIPass<'a>) {
		render_pass.0.set_vertex_buffer(0, self.buffer.slice(..));
		render_pass.0.set_bind_group(1, &self.bind_group, &[]);
		render_pass.0.draw(0..6, 0..1);
	}
}

pub struct UIPass<'a>(RenderPass<'a>);

pub trait RenderableUI<State> {
	fn render<'a>(&'a self, render_pass: UIPass<'a>, state: &'a State) -> UIPass<'a>;
}
