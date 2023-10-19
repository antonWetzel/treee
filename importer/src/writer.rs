use common::Project;
use math::Vector;
use std::{
	io::{Read, Write},
	mem::MaybeUninit,
	path::PathBuf,
};

use crate::ImporterError;

pub struct Writer {
	project_path: PathBuf,
	data_path: PathBuf,
	temp_path: PathBuf,
	index: usize,
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
		let mut data_path = path.clone();
		data_path.push("data");
		std::fs::create_dir_all(&data_path).unwrap();
		data_path.push("0.data");

		let mut temp_path = path.clone();
		temp_path.push("temp");
		std::fs::create_dir_all(&temp_path).unwrap();
		temp_path.push("0.data");

		path.push("project.epc");
		Ok(Self {
			project_path: path,
			data_path,
			temp_path,
			index: 1,
		})
	}

	pub fn save_project(&self, project: &Project) {
		project.save(&self.project_path);
	}

	pub fn next_index(&mut self) -> usize {
		self.index += 1;
		self.index
	}

	pub fn save(&self, index: usize, points: &[render::Point]) {
		let view = unsafe {
			std::slice::from_raw_parts(
				points as *const _ as *const u8,
				std::mem::size_of_val(points),
			)
		};
		let path = self.data_path.with_file_name(format!("{}.data", index));
		let mut file = std::fs::OpenOptions::new()
			.write(true)
			// .read(true)
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

	pub fn save_property(&self, index: usize, name: &str, property: &[u32]) {
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

	pub fn new_temp_file(&self, index: usize) -> std::fs::File {
		let path = self.temp_path.with_file_name(format!("{}.data", index));
		let file = std::fs::OpenOptions::new()
			.write(true)
			.read(true)
			.create(true)
			.open(path)
			.unwrap();
		file
	}

	pub fn load_temp_file(&self, index: usize, size: usize) -> Vec<Vector<3, f32>> {
		let path = self.temp_path.with_file_name(format!("{}.data", index));
		let mut file = std::fs::File::open(path).unwrap();
		unsafe {
			let mut data = Vec::<MaybeUninit<Vector<3, f32>>>::new();
			data.reserve_exact(size);
			data.set_len(size);
			let view = std::slice::from_raw_parts_mut(
				data.as_mut_ptr() as *mut u8,
				std::mem::size_of::<Vector<3, f32>>() * size,
			);
			file.read_exact(view).unwrap();
			std::mem::transmute(data)
		}
	}
}

impl Drop for Writer {
	fn drop(&mut self) {
		std::fs::remove_dir_all(&self.temp_path.parent().unwrap()).unwrap();
	}
}
