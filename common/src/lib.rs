use std::collections::HashMap;

use math::Vector;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize)]
pub struct Statistics {
	pub density: f32,
	pub max_neighbor_distance: f32,
	pub center: Vector<3, f32>,
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

pub struct Project {
	pub statistics: Statistics,
	pub level: u32,
	pub root: IndexNode,
	pub node_count: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct FlatProject {
	pub statistics: Statistics,
	pub level: u32,
	pub nodes: Vec<FlatNode>,
	pub node_count: u32,
}

impl Project {
	pub fn from_file<T: Into<String>>(path: T) -> Self {
		let file = std::fs::OpenOptions::new()
			.read(true)
			.open(path.into())
			.unwrap();
		let flat: FlatProject = ron::de::from_reader(file).unwrap();
		Project {
			statistics: flat.statistics,
			level: flat.level,
			root: deflatten(flat.nodes),
			node_count: flat.node_count,
		}
	}

	pub fn save<T: Into<String>>(&self, path: T) {
		let file = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.open(path.into())
			.unwrap();
		let flat = FlatProject {
			statistics: self.statistics,
			level: self.level,
			nodes: flatten(&self.root),
			node_count: self.node_count,
		};
		ron::ser::to_writer(file, &flat).unwrap();
	}
}

#[derive(Debug, Deserialize, Serialize)]
enum FlatData {
	Branch([Option<usize>; 8]),
	Leaf,
}

#[derive(Debug, Deserialize, Serialize)]
struct FlatNode {
	pub data: FlatData,
	pub position: Vector<3, f32>,
	pub size: f32,
	pub index: usize,
}

fn flatten(node: &IndexNode) -> Vec<FlatNode> {
	fn subflatten(node: &IndexNode, res: &mut Vec<FlatNode>) -> usize {
		let data = match &node.data {
			IndexData::Branch(children) => {
				let mut indices = [None; 8];
				for (i, child) in children.iter().enumerate() {
					indices[i] = child.as_ref().map(|node| subflatten(node, res))
				}
				FlatData::Branch(indices)
			},
			IndexData::Leaf() => FlatData::Leaf,
		};

		let index = res.len();
		res.push(FlatNode {
			data,
			position: node.position,
			size: node.size,
			index: node.index as usize,
		});
		index
	}
	let mut res = Vec::new();
	subflatten(node, &mut res);
	res
}

fn deflatten(nodes: Vec<FlatNode>) -> IndexNode {
	let mut results = HashMap::new();
	for (i, node) in nodes.into_iter().enumerate() {
		let data = match node.data {
			FlatData::Leaf => IndexData::Leaf(),
			FlatData::Branch(children) => IndexData::Branch(Box::new(
				children.map(|child| child.map(|idx| results.remove(&idx).unwrap())),
			)),
		};
		let node = IndexNode {
			data,
			position: node.position,
			size: node.size,
			index: node.index as u32,
		};
		results.insert(i, node);
	}
	assert_eq!(results.len(), 1);
	results.into_iter().next().unwrap().1
}
