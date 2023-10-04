use math::Vector;

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

const POS_OFFSET: wgpu::BufferAddress = memoffset::offset_of!(Point, position) as wgpu::BufferAddress;
const NORMAL_OFFSET: wgpu::BufferAddress = memoffset::offset_of!(Point, normal) as wgpu::BufferAddress;
const COLOR_OFFSET: wgpu::BufferAddress = memoffset::offset_of!(Point, color) as wgpu::BufferAddress;
const SIZE_OFFSET: wgpu::BufferAddress = memoffset::offset_of!(Point, size) as wgpu::BufferAddress;

impl Point {
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
			attributes: &[
				wgpu::VertexAttribute {
					offset: POS_OFFSET,
					shader_location: 1,
					format: wgpu::VertexFormat::Float32x3,
				},
				wgpu::VertexAttribute {
					offset: NORMAL_OFFSET,
					shader_location: 2,
					format: wgpu::VertexFormat::Float32x3,
				},
				wgpu::VertexAttribute {
					offset: COLOR_OFFSET,
					shader_location: 3,
					format: wgpu::VertexFormat::Float32x3,
				},
				wgpu::VertexAttribute {
					offset: SIZE_OFFSET,
					shader_location: 4,
					format: wgpu::VertexFormat::Float32,
				},
			],
		}
	}
}
