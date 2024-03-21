use std::num::NonZeroU32;

#[derive(Debug)]
pub struct Point {
	pub render: project::Point,

	pub segment: NonZeroU32,
	pub slice: u32,
	pub height: u32,
	pub curve: u32,
	pub classification: u32,
}

pub struct PointsCollection {
	pub render: Vec<project::Point>,

	pub slice: Vec<u32>,
	pub height: Vec<u32>,
	pub curve: Vec<u32>,
	pub segment: Vec<u32>,
	pub classification: Vec<u32>,
}

impl PointsCollection {
	pub fn new() -> Self {
		Self {
			render: Vec::new(),

			slice: Vec::new(),
			height: Vec::new(),
			curve: Vec::new(),
			segment: Vec::new(),
			classification: Vec::new(),
		}
	}

	pub fn from_points(points: &[Point]) -> Self {
		Self {
			render: points.iter().map(|p| p.render).collect(),
			slice: points.iter().map(|p| p.slice).collect(),
			height: points.iter().map(|p| p.height).collect(),
			curve: points.iter().map(|p| p.curve).collect(),
			segment: points.iter().map(|p| p.segment.get()).collect(),
			classification: points.iter().map(|p| p.classification).collect(),
		}
	}

	pub fn add(
		&mut self,
		render: project::Point,
		slice: u32,
		height: u32,
		curve: u32,
		segment: u32,
		classification: u32,
	) {
		self.render.push(render);
		self.slice.push(slice);
		self.height.push(height);
		self.curve.push(curve);
		self.segment.push(segment);
		self.classification.push(classification)
	}
}
