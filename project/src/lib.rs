use std::{
	fs::File,
	io::{BufReader, BufWriter, Read, Seek, Write},
	num::NonZeroU32,
	path::Path,
};

use nalgebra as na;
use serde::{Deserialize, Serialize};

pub const MAX_LEAF_SIZE: usize = 1 << 15;

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum IndexData {
	Branch(Box<[Option<IndexNode>; 8]>),
	Leaf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IndexNode {
	pub children: IndexData,
	pub position: na::Point<f32, 3>,
	pub size: f32,
	pub index: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Project {
	pub name: String,
	pub depth: u32,
	pub root: IndexNode,
	pub properties: Vec<Property>,

	pub segment_information: Vec<String>,
	pub segment_values: Vec<Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Property {
	pub storage_name: String,
	pub display_name: String,
	pub max: u32,
}

impl Project {
	pub fn from_file(path: impl AsRef<Path>) -> Self {
		let file = std::fs::OpenOptions::new().read(true).open(path).unwrap();
		serde_json::from_reader(BufReader::new(file)).unwrap()
	}

	pub fn empty() -> Self {
		Self {
			name: "No Project loaded".into(),
			depth: 0,
			root: IndexNode {
				children: IndexData::Leaf,
				position: na::Point::default(),
				size: 0.0,
				index: 0,
			},
			properties: vec![Property {
				display_name: String::from("None"),
				storage_name: String::from("none"),
				max: 1,
			}],
			segment_information: Vec::new(),
			segment_values: Vec::new(),
		}
	}

	pub fn save(&self, path: impl AsRef<Path>) {
		let file = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.open(path)
			.unwrap();
		serde_json::to_writer(BufWriter::new(file), self).unwrap();
	}

	pub fn segment(&self, index: NonZeroU32) -> &[Value] {
		let offset = (index.get() as usize - 1) * self.segment_information.len();
		&self.segment_values[offset..(offset + self.segment_information.len())]
	}
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Point {
	pub position: na::Point<f32, 3>,
	pub normal: na::SVector<f32, 3>,
	pub size: f32,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum Value {
	Index(NonZeroU32),
	Percent(f32),
	RelativeHeight { absolute: f32, percent: f32 },
}

impl std::fmt::Display for Value {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Index(v) => write!(f, "{}", v),
			Self::Percent(v) => write!(f, "{:.3}%", v * 100.0),
			Self::RelativeHeight { absolute, percent } => write!(f, "{:.2}m ({:.3}%)", absolute, percent * 100.0),
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
		file.set_len((size * 2 * std::mem::size_of::<u64>()) as u64)
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

	pub fn fake() -> Self {
		let mut file = tempfile::tempfile().unwrap();
		file.write_all(bytemuck::cast_slice(&[0u64, 0u64])).unwrap();
		Self { file, phantom: std::marker::PhantomData }
	}

	pub fn save(&mut self, idx: usize, data: &[T]) {
		self.file.seek(std::io::SeekFrom::End(0)).unwrap();
		let pos = [self.file.stream_position().unwrap(), data.len() as u64];
		self.file.write_all(bytemuck::cast_slice(data)).unwrap();
		self.file
			.seek(std::io::SeekFrom::Start(
				(idx * 2 * std::mem::size_of::<u64>()) as u64,
			))
			.unwrap();
		self.file.write_all(bytemuck::cast_slice(&pos)).unwrap();
	}

	pub fn read(&mut self, idx: usize) -> Vec<T> {
		let mut pos = [0u64, 0u64];
		self.file
			.seek(std::io::SeekFrom::Start(
				(idx * 2 * std::mem::size_of::<u64>()) as u64,
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

	pub fn sizes(&mut self, size: usize) -> Vec<[u64; 2]> {
		let mut buffer = vec![[0, 0]; size];
		self.file.seek(std::io::SeekFrom::Start(0)).unwrap();
		self.file
			.read_exact(bytemuck::cast_slice_mut(&mut buffer))
			.unwrap();
		buffer
	}
}
