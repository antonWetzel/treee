use std::collections::HashMap;
use std::io::{BufWriter, Read, Seek, Write};
use std::mem::MaybeUninit;

use common::{IndexData, IndexNode, Project, Statistics, MAX_LEAF_SIZE};
use indicatif::ProgressBar;
use math::Vector;
use math::{Dimension, Dimensions, X, Z};

use crate::calculations::{self};
use crate::{level_of_detail, Environment, Writer};

pub const MAX_NEIGHBORS: usize = 32 - 1;
pub const MAX_NEIGHBOR_DISTANCE_SCALE: f32 = 32.0;

#[derive(Debug)]
pub struct Leaf {
	file: BufWriter<std::fs::File>,
	size: usize,
}

impl Leaf {
	pub fn new(writer: &mut Writer, index: usize) -> Self {
		let file = writer.new_temp_file(index);
		file.set_len((MAX_LEAF_SIZE * std::mem::size_of::<Vector<3, f32>>()) as u64)
			.unwrap();
		Self { file: BufWriter::new(file), size: 0 }
	}

	pub fn add_position(&mut self, position: Vector<3, f32>) {
		let view = unsafe {
			std::slice::from_raw_parts(
				&position as *const _ as *const u8,
				std::mem::size_of::<Vector<3, f32>>(),
			)
		};
		self.file.write_all(view).unwrap();
		self.size += 1;
	}

	pub fn get_points(self) -> Vec<Vector<3, f32>> {
		let mut file = self.file.into_inner().unwrap();
		file.seek(std::io::SeekFrom::Start(0)).unwrap();

		unsafe {
			let mut data = Vec::<MaybeUninit<Vector<3, f32>>>::new();
			data.reserve_exact(self.size);
			data.set_len(self.size);
			let view = std::slice::from_raw_parts_mut(
				data.as_mut_ptr() as *mut u8,
				std::mem::size_of::<Vector<3, f32>>() * self.size,
			);
			file.read_exact(view).unwrap();
			std::mem::transmute(data)
		}
	}
}

#[derive(Debug)]
pub enum Data {
	Leaf(Leaf),
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

struct Adapter;
impl k_nearest::Adapter<3, f32, render::Point> for Adapter {
	fn get(point: &render::Point, dimension: Dimension) -> f32 {
		point.position[dimension]
	}
	fn get_all(point: &render::Point) -> [f32; 3] {
		point.position.data()
	}
}

impl Node {
	fn new(corner: Vector<3, f32>, size: f32, writer: &mut Writer) -> Self {
		let index = writer.next_index();
		Node {
			corner,
			size,
			data: Data::Leaf(Leaf::new(writer, index)),
			total_points: 0,
			index,
		}
	}

	fn insert_position(&mut self, position: Vector<3, f32>, writer: &mut Writer) {
		self.total_points += 1;
		match &mut self.data {
			Data::Branch { children, .. } => {
				let mut index = 0;
				let mut corner = self.corner;
				for dim in Dimensions(0..3) {
					if position[dim] >= self.corner[dim] + self.size / 2.0 {
						index += 1 << dim.0;
						corner[dim] += self.size / 2.0;
					}
				}

				match &mut children[index] {
					Some(v) => v.insert_position(position, writer),
					None => {
						let mut node = Node::new(corner, self.size / 2.0, writer);
						node.insert_position(position, writer);
						children[index] = Some(node);
					},
				}
			},
			Data::Leaf(leaf) => {
				if leaf.size < MAX_LEAF_SIZE {
					leaf.add_position(position);
					return;
				}
				let leaf = match std::mem::replace(&mut self.data, Data::Branch { children: Box::default() }) {
					Data::Leaf(leaf) => leaf,
					_ => unreachable!(),
				};

				let mut children: [Option<Self>; 8] = Default::default();
				let points = leaf.get_points();
				for position in points {
					let mut index = 0;
					for dim in X.to(Z) {
						if position[dim] >= self.corner[dim] + self.size / 2.0 {
							index += 1 << dim.0;
						}
					}
					match &mut children[index] {
						Some(v) => v.insert_position(position, writer),
						None => {
							let mut corner = self.corner;
							for dim in X.to(Z) {
								if position[dim] >= self.corner[dim] + self.size / 2.0 {
									corner[dim] += self.size / 2.0;
								}
							}
							let mut node = Node::new(corner, self.size / 2.0, writer);
							node.insert_position(position, writer);
							children[index] = Some(node);
						},
					}
				}
				self.data = Data::Branch { children: Box::new(children) };
				self.insert_position(position, writer);
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

	fn flatten(self, nodes: &mut Vec<FLatNode>) -> usize {
		let data = match self.data {
			Data::Branch { children } => {
				let mut indices = [None; 8];
				for (i, child) in children.into_iter().enumerate() {
					indices[i] = child.map(|node| node.flatten(nodes))
				}
				FlatData::Branch { children: indices }
			},
			Data::Leaf(leaf) => FlatData::Leaf { size: leaf.size },
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

	pub fn insert(&mut self, point: Vector<3, f32>, writer: &mut Writer) {
		self.root.insert_position(point, writer);
	}

	pub fn flatten(mut self, calculators: &[&str]) -> (FlatTree, Project) {
		let (tree, depth, node_count) = self.root.create_index_node();
		let mut nodes = Vec::with_capacity(node_count);
		let root = self.root.flatten_root();
		root.flatten(&mut nodes);
		let flat = FlatTree { nodes };
		let project = flat.genereate_project(depth, tree, node_count, calculators);
		(flat, project)
	}
}

#[derive(Debug)]
pub enum FlatData {
	Leaf { size: usize },
	Branch { children: [Option<usize>; 8] },
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
	pub fn genereate_project(&self, level: usize, root: IndexNode, node_count: usize, calculators: &[&str]) -> Project {
		let density = self.get_density();
		let density = density.sqrt();
		Project {
			statistics: Statistics {
				density,
				max_neighbor_distance: density * MAX_NEIGHBOR_DISTANCE_SCALE,
			},
			level: level as u32,
			root,
			node_count: node_count as u32,
			properties: calculators.iter().map(|&c| String::from(c)).collect(),
		}
	}

	fn get_density(&self) -> f32 {
		let mut total_size = 0.0;
		let mut density = 0.0;
		for node in &self.nodes {
			match &node.data {
				FlatData::Branch { .. } => {},
				&FlatData::Leaf { size } => {
					let area: f32 = node.size * node.size;
					let dens = area / size as f32;
					let new_size = total_size + size as f32;
					let weight = size as f32 / new_size;
					density = density * (1.0 - weight) + dens * weight;
					total_size = new_size;
				},
			}
		}
		density
	}

	pub fn calculate(self, writer: &Writer, project: &Project, environment: &Environment, progress: ProgressBar) {
		progress.reset();
		progress.set_length(project.node_count as u64);
		progress.set_prefix("Calculate:");

		let data = std::sync::Mutex::new(HashMap::new());
		let (sender, reciever) = crossbeam::channel::bounded::<(usize, FLatNode)>(4);

		std::thread::scope(|scope| {
			for _ in 0..num_cpus::get() {
				let reciever = reciever.clone();
				scope.spawn(|| {
					for (i, node) in reciever {
						let res = node.calculate(&data, writer, &project.statistics, environment);
						let mut data = data.lock().unwrap();
						data.insert(i, res);
						drop(data);
						progress.inc(1);
					}
				});
			}
			drop(reciever);

			for (i, node) in self.nodes.into_iter().enumerate() {
				sender.send((i, node)).unwrap();
			}
			drop(sender);
		});

		progress.finish();
		println!();
	}
}

impl FLatNode {
	fn calculate(
		mut self,
		data: &std::sync::Mutex<HashMap<usize, Vec<render::Point>>>,
		writer: &Writer,
		statistics: &Statistics,
		environment: &Environment,
	) -> Vec<render::Point> {
		match &mut self.data {
			FlatData::Branch { children } => {
				let mut points = Vec::with_capacity(8);
				for child in children.iter_mut().filter_map(|child| *child) {
					let lod = loop {
						let mut data = data.lock().unwrap();
						if let Some(v) = data.remove(&child) {
							break v;
						}
						drop(data);
						std::thread::yield_now();
					};
					points.push(lod);
				}

				let mut points = level_of_detail::grid(points, self.corner, self.size);
				writer.save(self.index, &points);

				let neighbors = Neighbors::new(&points, statistics);

				calculations::calculate(
					&mut points,
					false,
					&neighbors,
					environment,
					self.index,
					writer,
				);
				points
			},

			FlatData::Leaf { size } => {
				let positions = writer.load_temp_file(self.index, *size);
				let mut points = unsafe {
					let mut points = Vec::<render::Point>::new();
					points.reserve_exact(positions.len());
					points.set_len(positions.len());
					for (i, position) in positions.into_iter().enumerate() {
						points[i].position = position;
					}
					points
				};

				let neighbors = Neighbors::new(&points, statistics);

				calculations::calculate(
					&mut points,
					true,
					&neighbors,
					environment,
					self.index,
					writer,
				);

				writer.save(self.index, &points);
				points
			},
		}
	}
}

pub struct Neighbors(Vec<(usize, [(f32, usize); MAX_NEIGHBORS])>);

impl Neighbors {
	pub fn new(points: &[render::Point], statistics: &Statistics) -> Self {
		let kd_tree =
			k_nearest::KDTree::<3, f32, render::Point, Adapter, k_nearest::EuclideanDistanceSquared>::new(points);
		let mut neighbors = Vec::<(usize, [(f32, usize); MAX_NEIGHBORS])>::new();
		neighbors.reserve_exact(points.len());
		unsafe { neighbors.set_len(points.len()) };
		for (i, point) in points.iter().enumerate() {
			let neighbor = &mut neighbors[i];
			neighbor.0 = kd_tree.k_nearest(point, &mut neighbor.1, statistics.max_neighbor_distance);
		}
		Self(neighbors)
	}

	pub fn get(&self, index: usize) -> &[(f32, usize)] {
		&self.0[index].1[0..self.0[index].0]
	}
}
