use std::{
	collections::HashSet,
	fs::File,
	io::{ Read, Seek, Write },
	num::NonZeroU32,
	path::Path,
};

use math::Vector;
use serde::{ Deserialize, Serialize };


pub const MAX_LEAF_SIZE: usize = 1 << 15;


#[derive(Debug, Deserialize, Serialize)]
pub enum IndexData {
	Branch {
		children: Box<[Option<IndexNode>; 8]>,
	},
	Leaf {
		segments: HashSet<NonZeroU32>,
	},
}


#[derive(Debug, Deserialize, Serialize)]
pub struct IndexNode {
	pub data: IndexData,
	pub position: Vector<3, f32>,
	pub size: f32,
	pub index: u32,
}


#[derive(Debug, Deserialize, Serialize)]
pub struct Project {
	pub name: String,
	pub depth: u32,
	pub root: IndexNode,
	pub properties: Vec<(String, String)>,
}


impl Project {
	pub fn from_file(path: impl AsRef<Path>) -> Self {
		let file = std::fs::OpenOptions::new().read(true).open(path).unwrap();
		bincode::deserialize_from(file).unwrap()
	}


	pub fn save(&self, path: impl AsRef<Path>) {
		let file = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.open(path)
			.unwrap();
		bincode::serialize_into(file, self).unwrap();
	}
}


#[derive(Debug, Deserialize, Serialize)]
pub enum Value {
	Index(NonZeroU32),
	Percent(f32),
	RelativeHeight { absolute: f32, percent: f32 },
}


impl std::fmt::Display for Value {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Value::Index(v) => write!(f, "{}", v),
			Value::Percent(v) => write!(f, "{:.3}%", v * 100.0),
			Value::RelativeHeight { absolute, percent } => write!(f, "{:.2}m ({:.3}%)", absolute, percent),
		}
	}
}


pub struct DataFile<T>
where
	T: Copy + bytemuck::Pod,
{
	file: File,
	phantom: std::marker::PhantomData<T>,
}


impl<T> DataFile<T>
where
	T: Copy + bytemuck::Pod,
{
	pub fn new(size: usize, path: impl AsRef<Path>) -> Self {
		let file = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.open(path.as_ref())
			.unwrap();
		file.set_len((size * 2 * std::mem::size_of::<usize>()) as u64)
			.unwrap();
		Self { file, phantom: std::marker::PhantomData }
	}


	pub fn open(path: impl AsRef<Path>) -> Self {
		Self {
			file: std::fs::OpenOptions::new()
				.read(true)
				.open(path.as_ref())
				.unwrap(),
			phantom: std::marker::PhantomData,
		}
	}


	pub fn save(&mut self, idx: usize, data: &[T]) {
		self.file.seek(std::io::SeekFrom::End(0)).unwrap();
		let pos = [self.file.stream_position().unwrap(), data.len() as u64];
		self.file.write_all(bytemuck::cast_slice(data)).unwrap();
		self.file
			.seek(std::io::SeekFrom::Start(
				(idx * 2 * std::mem::size_of::<usize>()) as u64,
			))
			.unwrap();
		self.file.write_all(bytemuck::cast_slice(&pos)).unwrap();
	}


	pub fn read(&mut self, idx: usize) -> Vec<T> {
		let mut pos = [0u64, 0u64];
		self.file
			.seek(std::io::SeekFrom::Start(
				(idx * 2 * std::mem::size_of::<usize>()) as u64,
			))
			.unwrap();
		self.file
			.read_exact(bytemuck::cast_slice_mut(&mut pos))
			.unwrap();
		self.file.seek(std::io::SeekFrom::Start(pos[0])).unwrap();
		let mut buffer = vec![T::zeroed(); pos[1] as usize];
		self.file
			.read_exact(bytemuck::cast_slice_mut(&mut buffer))
			.unwrap();
		buffer
	}
}


#[derive(Serialize, Deserialize, Debug)]
pub struct Segment {
	pub values: Vec<(String, Value)>,
}


impl Segment {
	pub fn new(values: Vec<(String, Value)>) -> Self {
		Self { values }
	}


	pub fn save(&self, path: &Path) {
		let file = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.open(path)
			.unwrap();
		bincode::serialize_into(file, self).unwrap();
	}


	pub fn load(path: &Path) -> Self {
		let file = std::fs::OpenOptions::new().read(true).open(path).unwrap();
		bincode::deserialize_from(file).unwrap()
	}
}
