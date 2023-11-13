use common::Project;
use std::path::PathBuf;

use crate::ImporterError;

pub struct Writer {
	pub points: common::DataFile<render::Point>,
	pub slice: common::DataFile<u32>,
	pub curve: common::DataFile<u32>,
	pub sub_index: common::DataFile<u32>,
}

impl Writer {
	pub fn new(mut path: PathBuf, project: &Project) -> Result<Self, ImporterError> {
		if path.is_file() {
			return Err(ImporterError::OutputFolderIsFile);
		}
		if path.is_dir() {
			let mut project_path = path.clone();
			project_path.push("project.epc");
			if path.read_dir().into_iter().flatten().next().is_some() && !project_path.exists() {
				return Err(ImporterError::OutputFolderIsNotEmpty);
			}
			std::fs::remove_dir_all(&path).unwrap();
		}

		let size = project.root.index as usize + 1;
		std::fs::create_dir_all(&path).unwrap();
		path.push("project.epc");
		project.save(&path);

		path.set_file_name("points.data");
		let points = common::DataFile::new(size, &path);

		path.set_file_name("slice.data");
		let slice = common::DataFile::new(size, &path);

		path.set_file_name("curve.data");
		let curve = common::DataFile::new(size, &path);

		path.set_file_name("sub_index.data");
		let sub_index = common::DataFile::new(size, &path);

		Ok(Self { points, slice, curve, sub_index })
	}

	pub fn save(&mut self, index: u32, points: &[render::Point]) {
		self.points.save(index as usize, points);
	}
}
