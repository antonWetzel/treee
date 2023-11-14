use std::num::NonZeroU32;
use std::sync::Mutex;

use common::{IndexData, IndexNode, Project, MAX_LEAF_SIZE};
use crossbeam::atomic::AtomicCell;
use math::Vector;
use math::{X, Z};
use rayon::prelude::*;

use crate::cache::{Cache, CacheEntry, CacheIndex};
use crate::point::{Point, PointsCollection};
use crate::progress::Progress;
use crate::{level_of_detail, Writer};

#[derive(Debug)]
pub enum Data {
	Leaf {
		size: usize,
		segment: NonZeroU32,
		index: CacheIndex,
	},
	Branch {
		children: Box<[Option<Node>; 8]>,
	},
}

#[derive(Debug)]
pub struct Node {
	corner: Vector<3, f32>,
	size: f32,
	data: Data,
}

impl Node {
	fn new_branch(corner: Vector<3, f32>, size: f32) -> Self {
		Node {
			corner,
			size,
			data: Data::Branch { children: Box::default() },
		}
	}

	fn new_leaf(corner: Vector<3, f32>, size: f32, segment: NonZeroU32, cache: &mut Cache<Point>) -> Self {
		Node {
			corner,
			size,
			data: Data::Leaf {
				size: 0,
				segment,
				index: cache.new_entry(),
			},
		}
	}

	fn insert_position(&mut self, point: Point, segment: NonZeroU32, cache: &mut Cache<Point>) {
		fn insert_into_children(
			children: &mut [Option<Node>; 8],
			point: Point,
			corner: Vector<3, f32>,
			size: f32,
			segment: NonZeroU32,
			cache: &mut Cache<Point>,
		) {
			let mut index = 0;
			for dim in X.to(Z) {
				if point.render.position[dim] >= corner[dim] + size / 2.0 {
					index += 1 << dim.0;
				}
			}
			match &mut children[index] {
				Some(v) => v.insert_position(point, segment, cache),
				None => {
					let mut corner = corner;
					for dim in X.to(Z) {
						if index & (1 << dim.0) != 0 {
							corner[dim] += size / 2.0;
						}
					}
					let mut node = Node::new_leaf(corner, size / 2.0, segment, cache);
					node.insert_position(point, segment, cache);
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
				insert_into_children(&mut children, point, self.corner, self.size, segment, cache);
				Data::Branch { children }
			},
			Data::Leaf { mut size, segment: leaf_segment, index }
				if size < MAX_LEAF_SIZE && leaf_segment == segment =>
			{
				cache.add_point(&index, point);
				size += 1;
				Data::Leaf { size, segment, index }
			},
			Data::Leaf { size: _, segment: leaf_segment, index } if self.size > 0.1 => {
				let mut children: [Option<Self>; 8] = Default::default();
				let points = cache.read(index).read();
				for point in points {
					insert_into_children(
						&mut children,
						point,
						self.corner,
						self.size,
						leaf_segment,
						cache,
					)
				}
				insert_into_children(&mut children, point, self.corner, self.size, segment, cache);
				Data::Branch { children: Box::new(children) }
			},
			Data::Leaf { size, segment, index } => Data::Leaf { size, segment, index },
		});
	}

	fn flatten(self, nodes: &mut Vec<FLatNode>, cache: &mut Cache<Point>) -> (IndexNode, u32) {
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
					IndexData::Branch { children: Box::new(index_children) },
					max_level + 1,
				)
			},
			Data::Leaf { size, segment, index } => (
				FlatData::Leaf { size, data: cache.read(index) },
				IndexData::Leaf { segment },
				1,
			),
		};

		let index = nodes.len() as u32;
		nodes.push(FLatNode {
			corner: self.corner,
			size: self.size,
			data: flat_data,
			index,
		});
		(
			IndexNode {
				data: index_data,
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
	pub fn new(corner: Vector<3, f32>, size: f32) -> Self {
		Self { root: Node::new_branch(corner, size) }
	}

	pub fn insert(&mut self, point: Point, segment: NonZeroU32, cache: &mut Cache<Point>) {
		self.root.insert_position(point, segment, cache);
	}

	pub fn flatten(
		self,
		calculators: &[&str],
		segment_properties: &[&str],
		segment_values: Vec<common::Value>,
		name: String,
		mut cache: Cache<Point>,
	) -> (FlatTree, Project) {
		let mut nodes = Vec::new();
		let (tree, depth) = self.root.flatten(&mut nodes, &mut cache);
		let flat = FlatTree { nodes };
		let project = Project {
			name,
			depth,
			root: tree,
			properties: calculators.iter().map(|&c| String::from(c)).collect(),
			segment_properties: segment_properties
				.iter()
				.map(|&c| String::from(c))
				.collect(),
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
pub struct FLatNode {
	corner: Vector<3, f32>,
	size: f32,
	data: FlatData,
	index: u32,
}

#[derive(Debug)]
pub struct FlatTree {
	nodes: Vec<FLatNode>,
}

impl FlatTree {
	pub fn save(self, writer: Writer) {
		let progress = Mutex::new(Progress::new("Save Data", self.nodes.len()));

		let mut data = Vec::with_capacity(self.nodes.len());
		for _ in 0..self.nodes.len() {
			data.push(AtomicCell::new(None));
		}

		let writer = Mutex::new(writer);

		self.nodes
			.into_par_iter()
			.enumerate()
			.for_each(|(i, node)| {
				let res = node.save(&data, &writer);
				data[i].store(Some(res));
				progress.lock().unwrap().step();
			});

		progress.into_inner().unwrap().finish();
	}
}

impl FLatNode {
	fn save(self, data: &[AtomicCell<Option<PointsCollection>>], writer: &Mutex<Writer>) -> PointsCollection {
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

				let points = level_of_detail::grid(points, self.corner, self.size);

				{
					let mut writer = writer.lock().unwrap();
					writer.save(self.index, &points.render);
					writer.slice.save(self.index as usize, &points.slice);
					writer
						.sub_index
						.save(self.index as usize, &points.sub_index);
					writer.curve.save(self.index as usize, &points.curve);
				}

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

				let slice = data.iter().map(|p| p.slice).collect::<Vec<_>>();
				let sub_index = data.iter().map(|p| p.sub_index).collect::<Vec<_>>();
				let curve = data.iter().map(|p| p.curve).collect::<Vec<_>>();

				{
					let mut writer = writer.lock().unwrap();
					writer.save(self.index, &points);
					writer.curve.save(self.index as usize, &curve);
					writer.sub_index.save(self.index as usize, &sub_index);
					writer.slice.save(self.index as usize, &slice);
				}

				PointsCollection { render: points, slice, sub_index, curve }
			},
		}
	}
}
