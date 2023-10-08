use math::{Mat, Projection, Transform};
use wgpu::util::DeviceExt;

use crate::{Has, RenderPass, State};

#[derive(Clone, Copy)]
pub struct Camera3D {
	pub aspect: f32,
	pub fovy: f32,
	pub near: f32,
	pub far: f32,
}

impl Camera3D {
	pub fn new(aspect: f32, fovy: f32, near: f32, far: f32) -> Self {
		Self { aspect, fovy, near, far }
	}
}

pub struct Camera3DGPU {
	bind_group: wgpu::BindGroup,
}

impl Camera3DGPU {
	pub fn new(state: &impl Has<State>, camera: &crate::Camera3D, transform: &Transform<3, f32>) -> Self {
		let view = transform.inverse().as_matrix();
		let proj = Projection::create_perspective(camera.fovy, camera.aspect, camera.near, camera.far);

		let uniform = Uniform { view_proj: proj * view };
		let buffer = state
			.get()
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Camera Buffer"),
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
				label: Some("camera_bind_group"),
			});
		Self { bind_group }
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
				label: Some("camera_bind_group_layout"),
			})
	}

	pub fn bind<'a>(&'a self, render_pass: &mut RenderPass<'a>, index: u32) {
		render_pass.set_bind_group(index, &self.bind_group, &[]);
	}
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Uniform {
	pub view_proj: Mat<4, f32>,
}

unsafe impl bytemuck::Zeroable for Uniform {}
unsafe impl bytemuck::Pod for Uniform {}
