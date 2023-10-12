use math::Vector;
use wgpu::vertex_attr_array;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Point {
	pub position: Vector<3, f32>,
	pub normal: Vector<3, f32>,
	pub value: u32,
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
	const QUAD_ATTRIBUTES: [wgpu::VertexAttribute; 1] = vertex_attr_array![0 => Float32x2];
	const ATTRIBUTES: [wgpu::VertexAttribute; 4] =
		vertex_attr_array![1 => Float32x3, 2 => Float32x3, 3 => Uint32, 4 => Float32];

	pub fn quad_description<'a>() -> wgpu::VertexBufferLayout<'a> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<PointEdge>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &Self::QUAD_ATTRIBUTES,
		}
	}
	pub fn description<'a>() -> wgpu::VertexBufferLayout<'a> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<Point>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Instance,
			attributes: &Self::ATTRIBUTES,
		}
	}
}
