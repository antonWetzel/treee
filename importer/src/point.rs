#[derive(Debug)]
pub struct Point {
	pub render: render::Point,

	pub slice: u32,
}

pub struct PointsCollection {
	pub render: Vec<render::Point>,

	pub slice: Vec<u32>,
}

impl PointsCollection {
	pub fn new() -> Self {
		Self { render: Vec::new(), slice: Vec::new() }
	}

	pub fn add(&mut self, render: render::Point, slice: u32) {
		self.render.push(render);
		self.slice.push(slice);
	}
}
