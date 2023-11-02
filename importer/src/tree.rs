use common::{IndexData, IndexNode, Project, MAX_LEAF_SIZE};
use crossbeam::atomic::AtomicCell;
use indicatif::ProgressBar;
use math::Vector;
use math::{Dimensions, X, Z};
use rayon::prelude::*;

use crate::cache::{Cache, CacheEntry};
use crate::point::{Point, PointsCollection};
use crate::{level_of_detail, Writer};

pub const MAX_NEIGHBORS: usize = 32 - 1;
// pub const MAX_NEIGHBOR_DISTANCE_SCALE: f32 = 32.0;

#[derive(Debug)]
pub enum Data {
	Leaf(usize),
	Branch { children: Box<[Option<Node>; 8]> },
}

#[derive(Debug)]
pub struct Node {
	corner: Vector<3, f32>,
	size: f32,
	data: Data,
	index: usize,
	total_points: usize,
}

impl Node {
	fn new(corner: Vector<3, f32>, size: f32, writer: &mut Writer) -> Self {
		let index = writer.next_index();
		Node {
			corner,
			size,
			data: Data::Leaf(0),
			total_points: 0,
			index,
		}
	}

	fn insert_position(&mut self, point: Point, writer: &mut Writer, cache: &mut Cache<Point>) {
		self.total_points += 1;
		match &mut self.data {
			Data::Branch { children, .. } => {
				let mut index = 0;
				let mut corner = self.corner;
				for dim in Dimensions(0..3) {
					if point.render.position[dim] >= self.corner[dim] + self.size / 2.0 {
						index += 1 << dim.0;
						corner[dim] += self.size / 2.0;
					}
				}

				match &mut children[index] {
					Some(v) => v.insert_position(point, writer, cache),
					None => {
						let mut node = Node::new(corner, self.size / 2.0, writer);
						node.insert_position(point, writer, cache);
						children[index] = Some(node);
					},
				}
			},
			Data::Leaf(leaf) => {
				if *leaf < MAX_LEAF_SIZE {
					cache.add_point(self.index, point);
					*leaf += 1;
					return;
				}
				let mut children: [Option<Self>; 8] = Default::default();
				let points = cache.read(self.index).read();
				for point in points {
					let mut index = 0;
					for dim in X.to(Z) {
						if point.render.position[dim] >= self.corner[dim] + self.size / 2.0 {
							index += 1 << dim.0;
						}
					}
					match &mut children[index] {
						Some(v) => v.insert_position(point, writer, cache),
						None => {
							let mut corner = self.corner;
							for dim in X.to(Z) {
								if point.render.position[dim] >= self.corner[dim] + self.size / 2.0 {
									corner[dim] += self.size / 2.0;
								}
							}
							let mut node = Node::new(corner, self.size / 2.0, writer);
							node.insert_position(point, writer, cache);
							children[index] = Some(node);
						},
					}
				}
				self.data = Data::Branch { children: Box::new(children) };
				self.insert_position(point, writer, cache);
			},
		}
	}

	fn create_index_node(&mut self) -> (IndexNode, usize, usize) {
		let mut node_count = 1;
		let (data, level) = match &mut self.data {
			Data::Branch { children, .. } => {
				let mut index_children: [_; 8] = Default::default();
				let mut max_depth = 0;
				for (i, child) in children.iter_mut().enumerate() {
					if let Some(child) = child {
						let (child, depth, child_node_count) = child.create_index_node();
						index_children[i] = Some(child);
						max_depth = max_depth.max(depth);
						node_count += child_node_count;
					}
				}
				(IndexData::Branch(Box::new(index_children)), max_depth + 1)
			},
			Data::Leaf(..) => (IndexData::Leaf(), 1),
		};
		(
			IndexNode {
				data,
				position: self.corner,
				size: self.size,
				index: self.index as u32,
			},
			level,
			node_count,
		)
	}

	fn flatten_root(mut self) -> Self {
		match &mut self.data {
			Data::Branch { children } => {
				let count = children.iter().filter(|v| v.is_some()).count();
				if count == 1 {
					let children = std::mem::take(children);
					children.into_iter().find_map(|c| c).unwrap()
				} else {
					self
				}
			},
			Data::Leaf(_) => self,
		}
	}

	fn flatten(self, nodes: &mut Vec<FLatNode>, cache: &mut Cache<Point>) -> usize {
		let data = match self.data {
			Data::Branch { children } => {
				let mut indices = [None; 8];
				for (i, child) in children.into_iter().enumerate() {
					indices[i] = child.map(|node| node.flatten(nodes, cache))
				}
				FlatData::Branch { children: indices }
			},
			Data::Leaf(leaf) => FlatData::Leaf { size: leaf, data: cache.read(self.index) },
		};

		let index = nodes.len();
		nodes.push(FLatNode {
			corner: self.corner,
			size: self.size,
			data,
			index: self.index,
		});
		index
	}
}

pub struct Tree {
	root: Node,
}

impl Tree {
	pub fn new(writer: &mut Writer, corner: Vector<3, f32>, size: f32) -> Self {
		Self { root: Node::new(corner, size, writer) }
	}

	pub fn insert(&mut self, point: Point, writer: &mut Writer, cache: &mut Cache<Point>) {
		self.root.insert_position(point, writer, cache);
	}

	pub fn flatten(mut self, calculators: &[&str], name: String, mut cache: Cache<Point>) -> (FlatTree, Project) {
		let (tree, depth, node_count) = self.root.create_index_node();
		let mut nodes = Vec::with_capacity(node_count);
		let root = self.root.flatten_root();
		root.flatten(&mut nodes, &mut cache);
		let flat = FlatTree { nodes };
		let project = flat.genereate_project(depth, name, tree, node_count, calculators);
		(flat, project)
	}
}

#[derive(Debug)]
pub enum FlatData {
	Leaf {
		size: usize,
		data: CacheEntry<Point>,
	},
	Branch {
		children: [Option<usize>; 8],
	},
}

#[derive(Debug)]
pub struct FLatNode {
	corner: Vector<3, f32>,
	size: f32,
	data: FlatData,
	index: usize,
}

#[derive(Debug)]
pub struct FlatTree {
	nodes: Vec<FLatNode>,
}

impl FlatTree {
	pub fn genereate_project(
		&self,
		level: usize,
		name: String,
		root: IndexNode,
		node_count: usize,
		calculators: &[&str],
	) -> Project {
		Project {
			name,
			level: level as u32,
			root,
			node_count: node_count as u32,
			properties: calculators.iter().map(|&c| String::from(c)).collect(),
		}
	}

	pub fn save(self, writer: &Writer, project: &Project, progress: ProgressBar) {
		progress.reset();
		progress.set_length(project.node_count as u64);
		progress.set_prefix("Save Data:");

		let mut data = Vec::with_capacity(self.nodes.len());
		for _ in 0..self.nodes.len() {
			data.push(AtomicCell::new(None));
		}

		self.nodes
			.into_par_iter()
			.enumerate()
			.for_each(|(i, node)| {
				let res = node.save(&data, writer);
				data[i].store(Some(res));
				progress.inc(1);
			});

		progress.finish();
		println!();
	}
}

impl FLatNode {
	fn save(self, data: &[AtomicCell<Option<PointsCollection>>], writer: &Writer) -> PointsCollection {
		match self.data {
			FlatData::Branch { mut children } => {
				let mut points = Vec::with_capacity(8);
				for child in children.iter_mut().filter_map(|child| *child) {
					let lod = loop {
						if let Some(v) = data[child].take() {
							break v;
						}
						std::thread::yield_now();
					};
					points.push(lod);
				}

				let points = level_of_detail::grid(points, self.corner, self.size);
				writer.save(self.index, &points.render);

				writer.save_property(self.index, "slice", &points.slice);

				points
			},

			FlatData::Leaf { data, .. } => {
				let data = data.read();
				let points = unsafe {
					let mut points = Vec::<render::Point>::new();
					points.reserve_exact(data.len());
					points.set_len(data.len());
					for (i, d) in data.iter().enumerate() {
						points[i] = d.render;
					}
					points
				};
				writer.save(self.index, &points);

				let mut slice = Vec::with_capacity(data.len());
				for p in data {
					slice.push(p.slice);
				}
				writer.save_property(self.index, "slice", &slice);

				PointsCollection { render: points, slice }
			},
		}
	}
}
