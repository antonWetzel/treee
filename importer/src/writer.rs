use common::Project;
use std::io::{Read, Seek, Write};

pub struct Writer {
	path: String,
	index: usize,
}

impl Writer {
	pub fn new<T>(path: T) -> Self
	where
		T: Into<String>,
	{
		let path_string = path.into();
		let path = std::path::Path::new(&path_string);
		if path.is_dir() {
			std::fs::remove_dir_all(&path).unwrap();
		}
		std::fs::create_dir_all(&path).unwrap();
		std::fs::create_dir_all(format!("{}/data", path_string)).unwrap();

		Self { path: path_string, index: 1 }
	}

	pub fn save_project(&self, project: &Project) {
		project.save(format!("{}/project.epc", self.path));
	}

	pub fn next_index(&mut self) -> usize {
		self.index += 1;
		self.index
	}

	pub fn save(&self, index: usize, points: &[render::Point]) {
		let view = unsafe {
			std::slice::from_raw_parts(
				points as *const _ as *const u8,
				std::mem::size_of::<render::Point>() * points.len(),
			)
		};
		let path = format!("{}/data/{}.data", self.path, index);
		let mut file = std::fs::OpenOptions::new()
			.write(true)
			// .read(true)
			.create(true)
			.open(path)
			.unwrap();
		file.set_len(8 + view.len() as u64).unwrap();
		file.write_all(&points.len().to_le_bytes()).unwrap();
		file.write_all(view).unwrap();
	}

	pub fn new_file(&self, index: usize) -> std::fs::File {
		let path = format!("{}/data/{}.data", self.path, index);
		let file = std::fs::OpenOptions::new()
			.write(true)
			.read(true)
			.create(true)
			.open(path)
			.unwrap();
		file
	}

	pub fn load(&self, index: usize, size: usize) -> Vec<render::Point> {
		let path = format!("{}/data/{}.data", self.path, index);
		let mut file = std::fs::File::open(path).unwrap();
		file.seek(std::io::SeekFrom::Start(8)).unwrap();
		let mut data = Vec::with_capacity(size);
		unsafe {
			data.set_len(size);
			let view = std::slice::from_raw_parts_mut(
				data.as_mut_ptr() as *mut u8,
				std::mem::size_of::<render::Point>() * size,
			);
			file.read_exact(view).unwrap();
		};
		data
	}
}
