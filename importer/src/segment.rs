use math::{Vector, Y};

// only stub at the moment
// todo:
//  separate into segments
//  cache system
pub struct Segment {
	data: Vec<Vector<3, f32>>,
}

impl Segment {
	pub fn new() -> Self {
		Self { data: Vec::new() }
	}

	pub fn points(self) -> Vec<Vector<3, f32>> {
		self.data
	}
}

pub struct Segmenter {
	items: Vec<Segment>,
	split: f32,
}

impl Segmenter {
	pub fn new(split: f32) -> Self {
		Self {
			items: vec![Segment::new(), Segment::new()],
			split,
		}
	}

	pub fn add_point(&mut self, point: Vector<3, f32>) {
		if point[Y] < self.split {
			self.items[0].data.push(point);
		} else {
			self.items[1].data.push(point);
		}
	}

	pub fn result(self) -> Vec<Segment> {
		self.items
	}
}
