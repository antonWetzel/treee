use nalgebra as na;

use wgpu::util::DeviceExt;

use crate::{Has, State};

#[derive(Clone, Copy)]
pub enum Camera3D {
	Perspective {
		aspect: f32,
		fovy: f32,
		near: f32,
		far: f32,
	},
	Orthographic {
		aspect: f32,
		height: f32,
		near: f32,
		far: f32,
	},
}

impl Camera3D {
	pub fn projection(&self) -> na::Matrix4<f32> {
		match *self {
			Self::Perspective { aspect, fovy, near, far } => {
				na::Perspective3::new(aspect, fovy, near, far).to_homogeneous()
			},
			Self::Orthographic { aspect, height, near, far } => {
				na::Orthographic3::from_fov(aspect, height, near, far).to_homogeneous()
			},
		}
	}

	pub fn aspect(&self) -> f32 {
		match *self {
			Self::Perspective { aspect, .. } => aspect,
			Self::Orthographic { aspect, .. } => aspect,
		}
	}

	pub fn fovy(&self) -> f32 {
		match *self {
			Self::Perspective { fovy, .. } => fovy,
			Self::Orthographic { .. } => unreachable!(),
		}
	}

	pub fn set_aspect(&mut self, value: f32) {
		match self {
			Self::Perspective { aspect, .. } => *aspect = value,
			Self::Orthographic { aspect, .. } => *aspect = value,
		}
	}

	pub fn inside(&self, corner: na::Point<f32, 3>, size: f32, transform: na::Affine3<f32>) -> bool {
		let y = (self.fovy() / 2.0).tan();
		let x = y * self.aspect();

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
	pub fn new(state: &impl Has<State>, camera: &crate::Camera3D, transform: &na::Affine3<f32>) -> Self {
		let view = transform.inverse().to_homogeneous();
		let proj = camera.projection();

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

	pub fn get_bind_group(&self) -> &wgpu::BindGroup {
		&self.bind_group
	}
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct Uniform {
	pub view_proj: na::Matrix4<f32>,
}

unsafe impl bytemuck::Zeroable for Uniform {}

unsafe impl bytemuck::Pod for Uniform {}
