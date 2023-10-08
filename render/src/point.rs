use math::Vector;
use wgpu::vertex_attr_array;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Point {
	pub position: Vector<3, f32>,
	pub normal: Vector<3, f32>,
	pub color: Vector<3, f32>,
	pub size: f32,
}

unsafe impl bytemuck::Zeroable for Point {}
unsafe impl bytemuck::Pod for Point {}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PointEdge {
	pub position: Vector<2, f32>,
}

unsafe impl bytemuck::Zeroable for PointEdge {}
unsafe impl bytemuck::Pod for PointEdge {}

impl Point {
	const ATTRIBUTES: &[wgpu::VertexAttribute] =
		&vertex_attr_array![1 => Float32x3, 2 => Float32x3, 3 => Float32x3, 4 => Float32];

	pub fn quad_description<'a>() -> wgpu::VertexBufferLayout<'a> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<PointEdge>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &[wgpu::VertexAttribute {
				offset: 0,
				shader_location: 0,
				format: wgpu::VertexFormat::Float32x2,
			}],
		}
	}
	pub fn description<'a>() -> wgpu::VertexBufferLayout<'a> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<Point>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Instance,
			attributes: Self::ATTRIBUTES,
		}
	}
}
