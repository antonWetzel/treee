use math::Vector;
use winit::platform::run_return::EventLoopExtRunReturn;

use super::*;

pub struct State {
	pub(crate) device: wgpu::Device,
	pub(crate) queue: wgpu::Queue,
	pub(crate) instance: wgpu::Instance,
	pub(crate) adapter: wgpu::Adapter,
	pub(crate) surface_format: wgpu::TextureFormat,
}

pub struct Pipeline3D {
	pipeline: wgpu::RenderPipeline,
}

impl Pipeline3D {
	pub fn new(state: &State) -> Self {
		return Self {
			pipeline: state.create_pipeline(
				wgpu::include_wgsl!("../assets/shader_3d.wgsl"),
				&[Point::quad_description(), Point::description()],
				&[&gpu::Camera3D::get_layout(state)],
				Some("3d"),
				true,
			),
		};
	}

	pub fn render<'a, 'b: 'a, 'encoder, 'state: 'b, T>(
		&'a self,
		render_pass: &mut RenderPass<'encoder>,
		cam: &'a gpu::Camera3D,
		renderable: &'a T,
		state: &'state State,
	) where
		'a: 'encoder,
		T: Renderable,
	{
		render_pass.set_pipeline(&self.pipeline);
		render_pass.set_bind_group(0, &cam.bind_group, &[]);
		renderable.render(render_pass, state);
	}
}

impl State {
	pub async fn new() -> (Self, Runner) {
		env_logger::init();
		let event_loop = winit::event_loop::EventLoop::new();

		let window = winit::window::WindowBuilder::new()
			.with_visible(false)
			.build(&event_loop)
			.unwrap();

		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
			backends: wgpu::Backends::all(),
			dx12_shader_compiler: Default::default(),
		});

		let surface = unsafe { instance.create_surface(&window) }.unwrap();

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::default(),
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
			})
			.await
			.unwrap();

		let (device, queue) = adapter
			.request_device(
				&wgpu::DeviceDescriptor {
					features: wgpu::Features::empty(),
					limits: wgpu::Limits::default(),
					label: None,
				},
				None, // Trace path
			)
			.await
			.unwrap();

		let surface_caps = surface.get_capabilities(&adapter);
		let surface_format = surface_caps
			.formats
			.iter()
			.copied()
			.find(|f| f.describe().srgb)
			.unwrap_or(surface_caps.formats[0]);

		return (
			Self {
				instance,
				adapter,
				device,
				queue,
				surface_format,
			},
			Runner { event_loop },
		);
	}

	pub async fn additional_device(&self) -> wgpu::Device {
		let (device, _queue) = self
			.adapter
			.request_device(
				&wgpu::DeviceDescriptor {
					features: wgpu::Features::empty(),
					limits: wgpu::Limits::default(),
					label: None,
				},
				None,
			)
			.await
			.unwrap();
		device
	}

	fn create_pipeline(
		&self,
		wgsl: wgpu::ShaderModuleDescriptor,
		vertex_descriptions: &[wgpu::VertexBufferLayout],
		bind_group_layouts: &[&wgpu::BindGroupLayout],
		label: Option<&str>,
		depth: bool,
	) -> wgpu::RenderPipeline {
		let shader = self.device.create_shader_module(wgsl);
		let render_pipeline_layout = self
			.device
			.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("Render Pipeline Layout"),
				bind_group_layouts,
				push_constant_ranges: &[],
			});

		let render_pipeline = self
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
						format: self.surface_format,
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
			});
		return render_pipeline;
	}
}

pub struct Runner {
	pub event_loop: winit::event_loop::EventLoop<()>,
}

pub type WindowTarget = winit::event_loop::EventLoopWindowTarget<()>;

impl Runner {
	pub fn run<T: Game>(mut self, game: &mut T) -> i32 {
		self.event_loop
			.run_return(|event, _event_loop, control_flow| {
				*control_flow = match event {
					winit::event::Event::WindowEvent { ref event, window_id } => match event {
						winit::event::WindowEvent::CloseRequested => game.close_window(window_id),
						winit::event::WindowEvent::Resized(size) => {
							game.resize_window(window_id, [size.width, size.height].into())
						},
						winit::event::WindowEvent::ScaleFactorChanged { .. } => todo!(),
						winit::event::WindowEvent::KeyboardInput { input, .. } => {
							let key = match input.virtual_keycode {
								Some(key) => key,
								None => return,
							};
							game.key_changed(window_id, key, input.state)
						},
						winit::event::WindowEvent::MouseInput { state: button_state, button, .. } => {
							game.mouse_pressed(window_id, (*button).into(), *button_state)
						},
						winit::event::WindowEvent::MouseWheel { delta, .. } => {
							let delta = match delta {
								&winit::event::MouseScrollDelta::LineDelta(_, y) => -y,
								&winit::event::MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
							};
							game.mouse_wheel(delta)
						},
						winit::event::WindowEvent::CursorMoved { position, .. } => {
							let position = Vector::from([position.x, position.y]);
							game.mouse_moved(window_id, position)
						},
						&winit::event::WindowEvent::ModifiersChanged(modifiers) => {
							game.modifiers_changed(modifiers);
							ControlFlow::Poll
						},
						_ => ControlFlow::Poll,
					},
					winit::event::Event::RedrawRequested(window_id) => {
						game.render(window_id);
						ControlFlow::Poll
					},
					winit::event::Event::MainEventsCleared => {
						let control_flow = game.time();
						control_flow
					},
					_ => ControlFlow::Poll,
				}
			})
	}
}
