use common::Project;
use std::{io::Write, path::PathBuf};

use crate::ImporterError;

pub struct Writer {
	project_path: PathBuf,
	data_path: PathBuf,
}

impl Writer {
	pub fn new(mut path: PathBuf) -> Result<Self, ImporterError> {
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
		std::fs::create_dir_all(&path).unwrap();
		let mut data_path = path.clone();
		data_path.push("data");

		path.push("project.epc");
		Ok(Self { project_path: path, data_path })
	}

	pub fn save_project(&mut self, project: &Project) {
		std::fs::create_dir_all(&self.data_path).unwrap();
		self.data_path.push("0.data");
		project.save(&self.project_path);
	}

	pub fn save(&self, index: u32, points: &[render::Point]) {
		let view = unsafe {
			std::slice::from_raw_parts(
				points as *const _ as *const u8,
				std::mem::size_of_val(points),
			)
		};
		let path = self.data_path.with_file_name(format!("{}.data", index));
		let mut file = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.open(path)
			.unwrap();
		file.set_len(8 + view.len() as u64).unwrap();
		file.write_all(&(points.len() as u64).to_le_bytes())
			.unwrap();
		file.write_all(view).unwrap();
	}

	pub fn setup_property(&self, name: &str) {
		std::fs::create_dir_all(self.data_path.with_file_name(name)).unwrap();
	}

	pub fn save_property(&self, index: u32, name: &str, property: &[u32]) {
		let view = unsafe {
			std::slice::from_raw_parts(
				property as *const _ as *const u8,
				std::mem::size_of_val(property),
			)
		};
		let path = self
			.data_path
			.with_file_name(format!("{}/{}.data", name, index));
		let mut file = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.open(path)
			.unwrap();
		file.set_len(view.len() as u64).unwrap();
		file.write_all(view).unwrap();
	}
}
