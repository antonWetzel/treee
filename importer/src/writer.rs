use common::Project;
use std::path::{Path, PathBuf};

use crate::{point::PointsCollection, Error, Statistics};

pub struct Writer {
	path: PathBuf,
	pub points: common::DataFile<render::Point>,
	pub slice: common::DataFile<u32>,
	pub curve: common::DataFile<u32>,
	pub height: common::DataFile<u32>,
	pub segment: common::DataFile<u32>,
}

impl Writer {
	pub fn setup(path: &Path) -> Result<(), Error> {
		if path.is_file() {
			return Err(Error::OutputFolderIsFile);
		}
		if path.is_dir() {
			let mut project_path = path.to_path_buf();
			project_path.push("project.epc");
			if path.read_dir().into_iter().flatten().next().is_some() && !project_path.exists() {
				return Err(Error::OutputFolderIsNotEmpty);
			}
			std::fs::remove_dir_all(path).unwrap();
		}
		std::fs::create_dir_all(path).unwrap();
		Ok(())
	}

	pub fn new(mut path: PathBuf, size: usize) -> Self {
		path.push("temp.txt");

		path.set_file_name("points.data");
		let points = common::DataFile::new(size, &path);

		path.set_file_name("slice.data");
		let slice = common::DataFile::new(size, &path);

		path.set_file_name("curve.data");
		let curve = common::DataFile::new(size, &path);

		path.set_file_name("height.data");
		let height = common::DataFile::new(size, &path);

		path.set_file_name("segment.data");
		let segment = common::DataFile::new(size, &path);

		Self {
			points,
			slice,
			curve,
			height,
			segment,
			path,
		}
	}

	pub fn save(&mut self, index: usize, points: &PointsCollection) {
		self.points.save(index, &points.render);
		self.slice.save(index, &points.slice);
		self.height.save(index, &points.height);
		self.curve.save(index, &points.curve);
		self.segment.save(index, &points.segment);
	}

	pub fn save_project(&mut self, project: &Project) {
		self.path.set_file_name("project.epc");
		project.save(&self.path);
	}

	pub fn save_statistics(&mut self, statistics: Statistics) {
		self.path.set_file_name("statistics.json");
		let file = std::fs::File::create(&self.path).unwrap();
		serde_json::to_writer_pretty(file, &statistics).unwrap();
	}
}
