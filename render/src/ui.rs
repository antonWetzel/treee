use crate::{ Has, Render, RenderPass, State, Texture, Vertex2D };
use math::{ Transform, Vector, X, Y };
use wgpu::{ util::DeviceExt, SurfaceConfiguration };
use wgpu_text::{
	glyph_brush::{ ab_glyph::FontRef, BuiltInLineBreaker, HorizontalAlign, Layout, Section, Text, VerticalAlign },
	BrushBuilder,
	TextBrush,
};


pub struct UIState {
	pipeline: wgpu::RenderPipeline,
	sampler: wgpu::Sampler,
}


impl UIState {
	pub fn new(state: &impl Has<State>) -> Self {
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

		Self { sampler, pipeline }
	}


	pub fn sampler(&self) -> &wgpu::Sampler {
		&self.sampler
	}
}


pub struct UI<'a> {
	brush: TextBrush<FontRef<'a>>,
	projection: wgpu::BindGroup,
}


impl<'a> UI<'a> {
	pub fn new(state: &impl Has<State>, config: &SurfaceConfiguration, font_bytes: &'a [u8]) -> Self {
		let state: &State = state.get();
		let font = FontRef::try_from_slice(font_bytes).unwrap();

		let brush = BrushBuilder::using_font(font).build(&state.device, config.width, config.height, config.format);

		Self {
			brush,
			projection: Self::create_projection(state, config),
		}
	}


	pub fn resize(&mut self, state: &impl Has<State>, config: &SurfaceConfiguration) {
		let state: &State = state.get();
		self.brush
			.resize_view(config.width as f32, config.height as f32, &state.queue);
		self.projection = Self::create_projection(state, config);
	}


	pub fn queue(&mut self, state: &impl Has<State>, target: &impl UIElement) {
		let state = state.get();
		let mut collector = UICollector { data: Vec::new() };
		target.collect(&mut collector);
		self.brush
			.queue(&state.device, &state.queue, collector.data)
			.unwrap();
	}


	fn create_projection(state: &impl Has<State>, config: &SurfaceConfiguration) -> wgpu::BindGroup {
		let state: &State = state.get();
		let projection: Transform<2, f32> = Transform::translation([-1.0, 1.0].into())
			* Transform::scale([2.0 / config.width as f32, -2.0 / (config.height as f32)].into());
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


#[repr(transparent)]
pub struct UIPass<'a>(RenderPass<'a>);


impl<'a> UIPass<'a> {
	pub fn render(&mut self, value: &'a impl UIElement) {
		value.render(self);
	}
}


pub trait UIElement {
	fn render<'a>(&'a self, ui_pass: &mut UIPass<'a>);


	#[allow(unused)]
	fn collect<'a>(&'a self, collector: &mut UICollector<'a>);
}


impl<'a, T, S: Has<UIState>> Render<'a, (&'a UI<'a>, &'a S)> for T
where
	T: UIElement,
{
	fn render(&'a self, render_pass: &mut RenderPass<'a>, data: (&'a UI<'a>, &'a S)) {
		let (ui, state) = (data.0, data.1.get());
		ui.brush.draw(render_pass);
		render_pass.set_pipeline(&state.pipeline);
		render_pass.set_bind_group(0, &ui.projection, &[]);
		let ui_pass = unsafe {
			std::mem::transmute::<_, &mut UIPass<'a>>(render_pass)
		};
		self.render(ui_pass);
	}
}


pub struct UICollector<'a> {
	data: Vec<Section<'a>>,
}


impl<'a> UICollector<'a> {
	pub fn add_element(&mut self, element: &'a UIText) {
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
			layout: Layout::Wrap {
				line_breaker: BuiltInLineBreaker::UnicodeLineBreaker,
				h_align: element.horizontal,
				v_align: element.vertical,
			},
			..Default::default()
		});
	}
}


pub type UIHorizontalAlign = HorizontalAlign;
pub type UIVerticalAlign = VerticalAlign;


pub struct UIText {
	pub position: Vector<2, f32>,
	pub text: Vec<String>,
	pub font_size: f32,
	pub vertical: VerticalAlign,
	pub horizontal: HorizontalAlign,
}


impl UIText {
	pub fn new(
		text: Vec<String>,
		position: Vector<2, f32>,
		font_size: f32,
		horizontal: HorizontalAlign,
		vertical: VerticalAlign,
	) -> Self {
		Self {
			text,
			position,
			font_size,
			horizontal,
			vertical,
		}
	}
}


impl UIElement for UIText {
	fn render<'a>(&'a self, _ui_pass: &mut UIPass<'a>) { }


	fn collect<'a>(&'a self, collector: &mut UICollector<'a>) {
		collector.add_element(self)
	}
}


pub struct UIImage {
	bind_group: wgpu::BindGroup,
	buffer: wgpu::Buffer,
}


impl UIImage {
	pub fn new(
		state: &(impl Has<State> + Has<UIState>),
		position: Vector<2, f32>,
		size: Vector<2, f32>,
		texture: &Texture,
	) -> Self {
		let (state, ui_state): (&State, &UIState) = (state.get(), state.get());

		Self {
			buffer: Self::gpu_buffer(state, position, size),
			bind_group: Self::gpu_bind_group(state, ui_state, texture),
		}
	}


	pub fn update(&mut self, state: &(impl Has<State> + Has<UIState>), position: Vector<2, f32>, size: Vector<2, f32>) {
		self.buffer = Self::gpu_buffer(state.get(), position, size);
	}


	fn gpu_buffer(state: &State, position: Vector<2, f32>, size: Vector<2, f32>) -> wgpu::Buffer {
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

		state
			.get()
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Vertex Buffer"),
				contents: bytemuck::cast_slice(&vertices[..]),
				usage: wgpu::BufferUsages::VERTEX,
			})
	}


	fn gpu_bind_group(state: &State, ui_state: &UIState, texture: &Texture) -> wgpu::BindGroup {
		state.device.create_bind_group(&wgpu::BindGroupDescriptor {
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
		})
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
}


impl UIElement for UIImage {
	fn render<'a>(&'a self, ui_pass: &mut UIPass<'a>) {
		ui_pass.0.set_vertex_buffer(0, self.buffer.slice(..));
		ui_pass.0.set_bind_group(1, &self.bind_group, &[]);
		ui_pass.0.draw(0..6, 0..1);
	}


	fn collect<'a>(&'a self, _collector: &mut UICollector<'a>) { }
}
