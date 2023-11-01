#[derive(Debug)]
pub struct Point {
	pub render: render::Point,

	pub id: u32,
	pub curve: u32,
	pub height: u32,
	pub slice: u32,
}
