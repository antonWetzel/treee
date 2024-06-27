use nalgebra as na;
use wgpu::vertex_attr_array;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct PointEdge {
	pub position: na::Point2<f32>,
}

unsafe impl bytemuck::Zeroable for PointEdge {}
unsafe impl bytemuck::Pod for PointEdge {}

const BASE_ATTRIBUTES: [wgpu::VertexAttribute; 1] = vertex_attr_array![0 => Float32x2];
const ATTRIBUTES: [wgpu::VertexAttribute; 2] = vertex_attr_array![1 => Float32x3, 2 => Float32];
const PROPERTY_ATTRIBUTES: [wgpu::VertexAttribute; 1] = vertex_attr_array![4 => Uint32];

pub fn point_base_description<'a>() -> wgpu::VertexBufferLayout<'a> {
	wgpu::VertexBufferLayout {
		array_stride: std::mem::size_of::<PointEdge>() as wgpu::BufferAddress,
		step_mode: wgpu::VertexStepMode::Vertex,
		attributes: &BASE_ATTRIBUTES,
	}
}

pub fn point_description<'a>(step_mode: wgpu::VertexStepMode) -> wgpu::VertexBufferLayout<'a> {
	wgpu::VertexBufferLayout {
		array_stride: std::mem::size_of::<project::Point>() as wgpu::BufferAddress,
		step_mode,
		attributes: &ATTRIBUTES,
	}
}

pub fn point_property_description<'a>(step_mode: wgpu::VertexStepMode) -> wgpu::VertexBufferLayout<'a> {
	wgpu::VertexBufferLayout {
		array_stride: std::mem::size_of::<u32>() as wgpu::BufferAddress,
		step_mode,
		attributes: &PROPERTY_ATTRIBUTES,
	}
}
