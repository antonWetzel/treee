use std::num::NonZeroU32;

#[derive(Debug)]
pub struct Point {
	pub render: render::Point,

	pub segment: NonZeroU32,
	pub slice: u32,
	pub height: u32,
	pub curve: u32,
}

pub struct PointsCollection {
	pub render: Vec<render::Point>,

	pub slice: Vec<u32>,
	pub height: Vec<u32>,
	pub curve: Vec<u32>,
	pub segment: Vec<u32>,
}

impl PointsCollection {
	pub fn new() -> Self {
		Self {
			render: Vec::new(),

			slice: Vec::new(),
			height: Vec::new(),
			curve: Vec::new(),
			segment: Vec::new(),
		}
	}

	pub fn add(&mut self, render: render::Point, slice: u32, height: u32, curve: u32, segment: u32) {
		self.render.push(render);
		self.slice.push(slice);
		self.height.push(height);
		self.curve.push(curve);
		self.segment.push(segment);
	}
}
