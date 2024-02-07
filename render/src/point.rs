use math::Vector;
use wgpu::vertex_attr_array;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Point {
	pub position: Vector<3, f32>,
	pub normal: Vector<3, f32>,
	pub size: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PointEdge {
	pub position: Vector<2, f32>,
}

unsafe impl bytemuck::Zeroable for PointEdge {}

unsafe impl bytemuck::Pod for PointEdge {}

impl Point {
	const BASE_ATTRIBUTES: [wgpu::VertexAttribute; 1] = vertex_attr_array![0 => Float32x2];
	const ATTRIBUTES: [wgpu::VertexAttribute; 3] = vertex_attr_array![1 => Float32x3, 2 => Float32x3, 3 => Float32];
	const PROPERTY_ATTRIBUTES: [wgpu::VertexAttribute; 1] = vertex_attr_array![4 => Uint32];

	pub fn base_description<'a>() -> wgpu::VertexBufferLayout<'a> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<PointEdge>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &Self::BASE_ATTRIBUTES,
		}
	}

	pub fn description<'a>(step_mode: wgpu::VertexStepMode) -> wgpu::VertexBufferLayout<'a> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<Point>() as wgpu::BufferAddress,
			step_mode,
			attributes: &Self::ATTRIBUTES,
		}
	}

	pub fn property_description<'a>(step_mode: wgpu::VertexStepMode) -> wgpu::VertexBufferLayout<'a> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<u32>() as wgpu::BufferAddress,
			step_mode,
			attributes: &Self::PROPERTY_ATTRIBUTES,
		}
	}
}
