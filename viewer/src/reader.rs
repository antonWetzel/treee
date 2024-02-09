use std::path::PathBuf;

use common::DataFile;

pub struct Reader {
	points: DataFile<render::Point>,
	property: DataFile<u32>,
	path: PathBuf,
}

impl Reader {
	pub fn new(mut path: PathBuf, property: &str) -> Self {
		path.push("points.data");
		let points = DataFile::open(&path);
		path.set_file_name(format!("{}.data", property));
		let property = DataFile::open(&path);
		Self { points, property, path }
	}

	pub fn change_property(&mut self, property: &str) {
		self.path.set_file_name(format!("{}.data", property));
		self.property = DataFile::open(&self.path);
	}

	pub fn get_points(&mut self, index: usize) -> Vec<render::Point> {
		self.points.read(index)
	}

	pub fn get_property(&mut self, index: usize) -> Vec<u32> {
		self.property.read(index)
	}
}
