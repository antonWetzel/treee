use math::{Mat, Projection, Transform, Vector, W, X, Y, Z};
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
	pub fn projection(&self) -> Mat<4, f32> {
		match *self {
			Self::Perspective { aspect, fovy, near, far } => Projection::create_perspective(aspect, fovy, near, far),
			Self::Orthographic { aspect, height, near, far } => {
				Projection::create_orthographic(aspect, height, near, far)
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

	pub fn inside(&self, corner: Vector<3, f32>, size: f32, transform: Transform<3, f32>) -> bool {
		let matrix = self.projection() * transform.inverse().as_matrix();
		let points = [
			corner + [0.0, 0.0, 0.0].into(),
			corner + [0.0, 0.0, size].into(),
			corner + [0.0, size, 0.0].into(),
			corner + [0.0, size, size].into(),
			corner + [size, 0.0, 0.0].into(),
			corner + [size, 0.0, size].into(),
			corner + [size, size, 0.0].into(),
			corner + [size, size, size].into(),
		]
		.map(|point| matrix * Vector::new([point[X], point[Y], point[Z], 1.0]))
		.map(|point| Vector::new([point[X], point[Y], point[Z]]) / point[W]);

		let mut max = points[0];
		let mut min = points[0];
		for point in points.into_iter().skip(1) {
			max = max.max(point);
			min = min.min(point);
		}
		max[X] >= -1.0 && max[Y] >= -1.0 && max[Z] >= -1.0 && min[X] <= 1.0 && min[Y] <= 1.0 && min[Z] <= 1.0
	}
}

pub struct Camera3DGPU {
	bind_group: wgpu::BindGroup,
}

impl Camera3DGPU {
	pub fn new(state: &impl Has<State>, camera: &crate::Camera3D, transform: &Transform<3, f32>) -> Self {
		let view = transform.inverse().as_matrix();
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
	pub view_proj: Mat<4, f32>,
}

unsafe impl bytemuck::Zeroable for Uniform {}
unsafe impl bytemuck::Pod for Uniform {}
