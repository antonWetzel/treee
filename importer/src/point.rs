use std::num::NonZeroU32;

#[derive(Debug)]
pub struct Point {
	pub render: render::Point,

	pub segment: NonZeroU32,
	pub slice: u32,
	pub sub_index: u32,
	pub curve: u32,
}

pub struct PointsCollection {
	pub render: Vec<render::Point>,

	pub slice: Vec<u32>,
	pub sub_index: Vec<u32>,
	pub curve: Vec<u32>,
}

impl PointsCollection {
	pub fn new() -> Self {
		Self {
			render: Vec::new(),

			slice: Vec::new(),
			sub_index: Vec::new(),
			curve: Vec::new(),
		}
	}

	pub fn add(&mut self, render: render::Point, slice: u32, sub_index: u32, curve: u32) {
		self.render.push(render);
		self.slice.push(slice);
		self.sub_index.push(sub_index);
		self.curve.push(curve);
	}
}
