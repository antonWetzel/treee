use std::path::Path;

use math::Vector;
use serde::{Deserialize, Serialize};

pub const MAX_LEAF_SIZE: usize = 1 << 15;

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize)]
pub struct Statistics {
	pub density: f32,
	pub max_neighbor_distance: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum IndexData {
	Branch(Box<[Option<IndexNode>; 8]>),
	Leaf(),
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
	pub statistics: Statistics,
	pub level: u32,
	pub root: IndexNode,
	pub node_count: u32,
	pub properties: Vec<String>,
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
