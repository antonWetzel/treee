use common::Project;
use std::{
	io::{Read, Seek, Write},
	mem::MaybeUninit,
	path::PathBuf,
};

pub struct Writer {
	project_path: PathBuf,
	data_path: PathBuf,
	index: usize,
}

impl Writer {
	pub fn new(mut path: PathBuf) -> Self {
		if path.is_dir() {
			std::fs::remove_dir_all(&path).unwrap();
		}
		let mut data_path = path.clone();
		data_path.push("data");
		std::fs::create_dir_all(&data_path).unwrap();
		data_path.push("0.data");

		path.push("project.epc");
		Self { project_path: path, data_path, index: 1 }
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
		std::fs::create_dir_all(self.data_path.with_file_name(format!("{}", name))).unwrap();
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

	pub fn new_file(&self, index: usize) -> std::fs::File {
		let path = self.data_path.with_file_name(format!("{}.data", index));
		let file = std::fs::OpenOptions::new()
			.write(true)
			.read(true)
			.create(true)
			.open(path)
			.unwrap();
		file
	}

	pub fn load(&self, index: usize, size: usize) -> Vec<render::Point> {
		let path = self.data_path.with_file_name(format!("{}.data", index));
		let mut file = std::fs::File::open(path).unwrap();
		file.seek(std::io::SeekFrom::Start(8)).unwrap();
		unsafe {
			let mut data = Vec::<MaybeUninit<render::Point>>::new();
			data.reserve_exact(size);
			data.set_len(size);
			let view = std::slice::from_raw_parts_mut(
				data.as_mut_ptr() as *mut u8,
				std::mem::size_of::<render::Point>() * size,
			);
			file.read_exact(view).unwrap();
			std::mem::transmute(data)
		}
	}
}
