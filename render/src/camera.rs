use math::Vector;

#[derive(Clone, Copy)]
pub struct Camera3D {
	pub aspect: f32,
	pub fovy: f32,
	pub near: f32,
	pub far: f32,
}

impl Camera3D {
	pub fn new(aspect: f32, fovy: f32, near: f32, far: f32) -> Self {
		return Self { aspect, fovy, near, far };
	}
}

#[derive(Clone, Copy)]
pub struct Camera2D {
	pub window_size: Vector<2, u32>,
	pub zoom: f32,
}

impl Camera2D {
	pub fn new(window_size: Vector<2, u32>, zoom: f32) -> Self {
		return Self { window_size, zoom };
	}
}
