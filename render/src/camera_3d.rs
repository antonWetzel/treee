use nalgebra as na;

use wgpu::util::DeviceExt;

use crate::State;

#[derive(Clone, Copy)]
pub struct Camera3D {
	pub aspect: f32,
	pub fovy: f32,
	pub near: f32,
	pub far: f32,
}

impl Camera3D {
	pub fn projection(&self) -> na::Matrix4<f32> {
		na::Perspective3::new(self.aspect, self.fovy, self.near, self.far).to_homogeneous()
	}

	pub fn inside(&self, corner: na::Point<f32, 3>, size: f32, transform: na::Affine3<f32>) -> bool {
		let y = (self.fovy / 2.0).tan();
		let x = y * self.aspect;

		let planes = [
			na::vector![-1.0, 0.0, x],
			na::vector![1.0, 0.0, x],
			na::vector![0.0, -1.0, y],
			na::vector![0.0, 1.0, y],
		];

		let t = transform.inverse();
		let points = [
			na::vector![0.0, 0.0, 0.0],
			na::vector![0.0, 0.0, size],
			na::vector![0.0, size, 0.0],
			na::vector![0.0, size, size],
			na::vector![size, 0.0, 0.0],
			na::vector![size, 0.0, size],
			na::vector![size, size, 0.0],
			na::vector![size, size, size],
		]
		.map(|point| corner + point)
		.map(|point| t * point);

		for plane in planes {
			if points.iter().copied().all(|p| p.coords.dot(&plane) > 0.0) {
				return false;
			}
		}
		true
	}
}

pub struct Camera3DGPU {
	bind_group: wgpu::BindGroup,
}

impl Camera3DGPU {
	pub fn new(state: &State, camera: &crate::Camera3D, transform: &na::Affine3<f32>) -> Self {
		let view = transform.inverse().to_homogeneous();
		let proj = camera.projection();

		let uniform = Uniform { view, proj };
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
		Self { bind_group }
	}

	pub fn get_layout(state: &State) -> wgpu::BindGroupLayout {
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

	pub fn get_bind_group(&self) -> &wgpu::BindGroup {
		&self.bind_group
	}
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Uniform {
	pub view: na::Matrix4<f32>,
	pub proj: na::Matrix4<f32>,
}

unsafe impl bytemuck::Zeroable for Uniform {}

unsafe impl bytemuck::Pod for Uniform {}
