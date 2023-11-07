use std::{num::NonZeroU32, path::Path};

use math::Vector;
use serde::{Deserialize, Serialize};

pub const MAX_LEAF_SIZE: usize = 1 << 15;

#[derive(Debug, Deserialize, Serialize)]
pub enum IndexData {
	Branch {
		children: Box<[Option<IndexNode>; 8]>,
	},
	Leaf {
		segment: NonZeroU32,
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
	pub properties: Vec<String>,

	pub segment_properties: Vec<String>,
	pub segment_values: Vec<Value>,
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

	pub fn get_segment_values(&self, index: usize) -> &[Value] {
		let l = self.segment_properties.len();
		&self.segment_values[(index * l)..(index * l + l)]
	}
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Value {
	Index(NonZeroU32),
	Percent(f32),
}

impl std::fmt::Display for Value {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Value::Index(v) => write!(f, "{}", v),
			Value::Percent(v) => write!(f, "{:.3}%", v),
		}
	}
}
