use math::Vector;

#[derive(Debug)]
pub struct DataPoint {
	pub position: Vector<3, f32>,
	pub color: Vector<3, f32>,
}

impl Default for DataPoint {
	fn default() -> Self {
		Self {
			position: [0.0, 0.0, 0.0].into(),
			color: [1.0, 1.0, 1.0].into(),
		}
	}
}
