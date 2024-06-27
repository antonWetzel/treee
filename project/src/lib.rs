use std::{
	io::{BufReader, BufWriter},
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
	pub fn from_file(path: impl AsRef<Path>) -> Option<Self> {
		let file = std::fs::OpenOptions::new().read(true).open(path).ok()?;
		serde_json::from_reader(BufReader::new(file)).ok()
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
			.truncate(true)
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
	pub size: f32,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum Value {
	Index(NonZeroU32),
	Percent(f32),
	RelativeHeight { absolute: f32, percent: f32 },
	Meters(f32),
	MetersSquared(f32),
	AbsolutePosition(f64),
	Degrees(f64),
}

impl std::fmt::Display for Value {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Index(v) => write!(f, "{}", v),
			Self::Percent(v) => write!(f, "{:.3}%", v * 100.0),
			Self::RelativeHeight { absolute, percent } => write!(f, "{:.2}m ({:.3}%)", absolute, percent * 100.0),
			Self::Meters(value) => write!(f, "{:.2}m", value),
			Self::MetersSquared(value) => write!(f, "{:.2}m²", value),
			Self::AbsolutePosition(value) => write!(f, "{:.5}", value),
			&Self::Degrees(deg) => {
				let min = deg.fract() * if deg >= 0.0 { 60.0 } else { -60.0 };
				let deg = deg.trunc() as isize;
				let (min, sec) = (min.trunc() as isize, min.fract() * 60.0);
				write!(f, "{}°{}'{:.2}\"", deg, min, sec)
			},
		}
	}
}
