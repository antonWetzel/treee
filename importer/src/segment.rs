use math::Vector;

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
}

impl Segmenter {
	pub fn new() -> Self {
		Self { items: vec![Segment::new()] }
	}

	pub fn add_point(&mut self, point: Vector<3, f32>) {
		self.items[0].data.push(point);
	}

	pub fn result(self) -> Vec<Segment> {
		self.items
	}
}
