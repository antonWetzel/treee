use std::collections::HashSet;
use std::num::NonZeroU32;
use std::sync::Mutex;

use crossbeam::atomic::AtomicCell;
use nalgebra as na;
use project::{IndexData, IndexNode, Project, Property, MAX_LEAF_SIZE};
use rayon::prelude::*;

use crate::cache::{Cache, CacheEntry, CacheIndex};
use crate::point::{Point, PointsCollection};
use crate::progress::Progress;
use crate::{level_of_detail, Settings, Statistics, Writer};

#[derive(Debug)]
pub enum Data {
	Leaf {
		size: usize,
		segments: HashSet<NonZeroU32>,
		index: CacheIndex<Point>,
	},
	Branch {
		children: Box<[Option<Node>; 8]>,
	},
}

#[derive(Debug)]
pub struct Node {
	corner: na::Point3<f32>,
	size: f32,
	data: Data,
}

impl Node {
	fn new_branch(corner: na::Point3<f32>, size: f32) -> Self {
		Self {
			corner,
			size,
			data: Data::Branch { children: Box::default() },
		}
	}

	fn new_leaf(corner: na::Point3<f32>, size: f32, cache: &mut Cache) -> Self {
		Self {
			corner,
			size,
			data: Data::Leaf {
				size: 0,
				segments: HashSet::new(),
				index: cache.new_entry(),
			},
		}
	}

	fn insert_position(&mut self, point: Point, cache: &mut Cache) {
		fn insert_into_children(
			children: &mut [Option<Node>; 8],
			point: Point,
			corner: na::Point3<f32>,
			size: f32,
			cache: &mut Cache,
		) {
			let mut index = 0;
			for dim in 0..3 {
				if point.render.position[dim] >= corner[dim] + size / 2.0 {
					index += 1 << dim;
				}
			}
			match &mut children[index] {
				Some(v) => v.insert_position(point, cache),
				None => {
					let mut corner = corner;
					for dim in 0..3 {
						if index & (1 << dim) != 0 {
							corner[dim] += size / 2.0;
						}
					}
					let mut node = Node::new_leaf(corner, size / 2.0, cache);
					node.insert_position(point, cache);
					children[index] = Some(node);
				},
			}
		}

		fn update_value<T>(value: &mut T, update: impl FnOnce(T) -> T) {
			unsafe {
				let v = std::ptr::read(value);
				let v = update(v);
				std::ptr::write(value, v);
			}
		}

		update_value(&mut self.data, |data| match data {
			Data::Branch { mut children, .. } => {
				insert_into_children(&mut children, point, self.corner, self.size, cache);
				Data::Branch { children }
			},
			Data::Leaf { mut size, mut segments, index } if size < MAX_LEAF_SIZE => {
				segments.insert(point.segment);
				cache.add_value(&index, point);
				size += 1;
				Data::Leaf { size, segments, index }
			},
			Data::Leaf { size: _, segments: _, index } => {
				let mut children: [Option<Self>; 8] = Default::default();
				let points = cache.read(index).read();
				for point in points {
					insert_into_children(&mut children, point, self.corner, self.size, cache)
				}
				insert_into_children(&mut children, point, self.corner, self.size, cache);
				Data::Branch { children: Box::new(children) }
			},
		});
	}

	fn flatten(self, nodes: &mut Vec<FlatNode>, cache: &mut Cache) -> (IndexNode, u32) {
		let (flat_data, index_data, depth) = match self.data {
			Data::Branch { children } => {
				let mut indices = [None; 8];
				let mut index_children = [None, None, None, None, None, None, None, None];
				let mut max_level = 0;
				for (i, child) in children.into_iter().enumerate() {
					if let Some(child) = child {
						let (index_node, level) = child.flatten(nodes, cache);
						indices[i] = Some(index_node.index);
						index_children[i] = Some(index_node);
						max_level = max_level.max(level);
					}
				}
				(
					FlatData::Branch { children: indices },
					IndexData::Branch(Box::new(index_children)),
					max_level + 1,
				)
			},
			Data::Leaf { size, segments: _, index } => (
				FlatData::Leaf { size, data: cache.read(index) },
				IndexData::Leaf,
				1,
			),
		};

		let index = nodes.len() as u32;
		nodes.push(FlatNode {
			corner: self.corner,
			size: self.size,
			data: flat_data,
			index,
		});
		(
			IndexNode {
				children: index_data,
				position: self.corner,
				size: self.size,
				index,
			},
			depth,
		)
	}
}

pub struct Tree {
	root: Node,
}

impl Tree {
	pub fn new(corner: na::Point3<f32>, size: f32) -> Self {
		Self { root: Node::new_branch(corner, size) }
	}

	pub fn insert(&mut self, point: Point, cache: &mut Cache) {
		self.root.insert_position(point, cache);
	}

	pub fn flatten(
		self,
		properties: Vec<Property>,
		name: String,
		mut cache: Cache,
		segment_information: Vec<String>,
		segment_values: Vec<project::Value>,
	) -> (FlatTree, Project) {
		let mut nodes = Vec::new();
		let (tree, depth) = self.root.flatten(&mut nodes, &mut cache);
		let flat = FlatTree { nodes };
		let project = Project {
			name,
			depth,
			root: tree,
			properties,
			segment_information,
			segment_values,
		};

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
		children: [Option<u32>; 8],
	},
}

#[derive(Debug)]
pub struct FlatNode {
	corner: na::Point3<f32>,
	size: f32,
	data: FlatData,
	index: u32,
}

#[derive(Debug)]
pub struct FlatTree {
	nodes: Vec<FlatNode>,
}

impl FlatTree {
	pub fn save(self, writer: Writer, settings: &Settings, statistics: Statistics) {
		let progress = Progress::new("Save Data", self.nodes.len());

		let mut data = Vec::with_capacity(self.nodes.len());
		for _ in 0..self.nodes.len() {
			data.push(AtomicCell::new(None));
		}

		let state = Mutex::new((progress, writer, statistics));

		self.nodes
			.into_par_iter()
			.enumerate()
			.for_each(|(i, node)| {
				let is_leaf = matches!(&node.data, FlatData::Leaf { .. });
				let index = node.index as usize;
				let points = node.save(&data, settings);
				let mut state = state.lock().unwrap();
				state.0.step();
				{
					let writer = &mut state.1;
					writer.save(index, &points);
				}
				if is_leaf {
					state.2.leaf_points += points.render.len();
				} else {
					state.2.branch_points += points.render.len();
				}
				drop(state);
				data[i].store(Some(points));
			});
		let (progress, mut writer, mut statistics) = state.into_inner().unwrap();
		statistics.times.lods = progress.finish();
		writer.save_statistics(statistics);
	}
}

impl FlatNode {
	fn save(self, data: &[AtomicCell<Option<PointsCollection>>], settings: &Settings) -> PointsCollection {
		match self.data {
			FlatData::Branch { mut children } => {
				let mut points = Vec::with_capacity(8);
				for child in children.iter_mut().filter_map(|child| *child) {
					let lod = loop {
						if let Some(v) = data[child as usize].take() {
							break v;
						}
						std::thread::yield_now();
					};
					points.push(lod);
				}
				level_of_detail::grid(points, self.corner, self.size, settings)
			},

			FlatData::Leaf { data, .. } => {
				let data = data.read();
				let points = unsafe {
					let mut points = Vec::<project::Point>::new();
					points.reserve_exact(data.len());
					points.set_len(data.len());
					for (i, d) in data.iter().enumerate() {
						points[i] = d.render;
					}
					points
				};

				PointsCollection {
					render: points,
					slice: data.iter().map(|p| p.slice).collect::<Vec<_>>(),
					height: data.iter().map(|p| p.height).collect::<Vec<_>>(),
					curve: data.iter().map(|p| p.curve).collect::<Vec<_>>(),
					segment: data.iter().map(|p| p.segment.get()).collect::<Vec<_>>(),
					classification: data.iter().map(|p| p.classification).collect::<Vec<_>>(),
				}
			},
		}
	}
}
