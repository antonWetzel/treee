use math::Vector;

#[derive(Debug)]
pub struct DataPoint {
	pub position: Vector<3, f32>,
	pub value: u32,
}
