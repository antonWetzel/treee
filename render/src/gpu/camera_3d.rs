use math::{Mat, Projection, Transform};

use super::*;

pub struct Camera3D {
	pub(crate) bind_group: wgpu::BindGroup,
}

impl Camera3D {
	pub fn new_empty(state: &crate::State) -> Self {
		return Self::create(state, Mat::default());
	}

	pub fn new(state: &crate::State, camera: &crate::Camera3D, transform: &Transform<3, f32>) -> Self {
		let view = transform.inverse().as_matrix();
		let proj = Projection::create_perspective(camera.fovy, camera.aspect, camera.near, camera.far);
		return Self::create(state, proj * view);
	}

	fn create(state: &crate::State, matrix: Mat<4, f32>) -> Self {
		let uniform = Uniform { view_proj: matrix };
		let buffer = state
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Camera Buffer"),
				contents: bytemuck::cast_slice(&[uniform]),
				usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			});

		let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &Self::get_layout(state),
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: buffer.as_entire_binding(),
			}],
			label: Some("camera_bind_group"),
		});
		return Self { bind_group };
	}

	pub fn get_layout(state: &crate::State) -> wgpu::BindGroupLayout {
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
				label: Some("camera_bind_group_layout"),
			})
	}
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Uniform {
	pub view_proj: Mat<4, f32>,
}

unsafe impl bytemuck::Zeroable for Uniform {}
unsafe impl bytemuck::Pod for Uniform {}
