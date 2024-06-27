use std::{collections::HashMap, ops::Not, sync::Mutex};

use dashmap::DashMap;
use nalgebra as na;

use crate::task::Task;

const MAX_POINTS: usize = 1 << 14;

pub struct Octree {
	pub generation: usize,
	corner: na::Point3<f32>,
	size: f32,
	nodes: DashMap<usize, Node>,
}

enum Node {
	Branch {},
	Leaf {
		points: Vec<na::Point3<f32>>,
		property: Vec<u32>,
		dirty: bool,
	},
}

impl Octree {
	pub fn new(corner: na::Point3<f32>, size: f32, generation: usize) -> Self {
		Self {
			corner,
			size,
			nodes: DashMap::new(),
			generation,
		}
	}

	pub fn insert(&self, point: na::Point3<f32>, chunk: usize, sender: &crossbeam::channel::Sender<Task>) {
		let idx = 0;
		let corner = self.corner;
		let size = self.size;
		self.insert_with(point, chunk as u32, corner, size, idx, 0, sender);
	}

	fn insert_with(
		&self,
		point: na::Point3<f32>,
		chunk: u32,
		mut corner: na::Point3<f32>,
		mut size: f32,
		mut idx: usize,
		mut dim: usize,
		sender: &crossbeam::channel::Sender<Task>,
	) {
		loop {
			let mut entry = self.nodes.entry(idx).or_insert_with(|| Node::Leaf {
				points: Vec::new(),
				property: Vec::new(),
				dirty: false,
			});
			let node = entry.value_mut();
			match node {
				Node::Branch {} => {
					drop(entry);
				},
				Node::Leaf { points, property, dirty } => {
					if dirty.not() {
						*dirty = true;
						sender.send(Task::Update(idx)).unwrap();
					}
					if points.len() < MAX_POINTS {
						points.push(point);
						property.push(chunk);
						break;
					}
					let points = std::mem::take(points);
					let property = std::mem::take(property);
					*node = Node::Branch {};
					drop(entry);

					for (point, chunk) in points.into_iter().zip(property) {
						self.insert_with(point, chunk, corner, size, idx, dim, sender);
					}
					self.insert_with(point, chunk, corner, size, idx, dim, sender);
				},
			}
			if dim == 0 {
				size /= 2.0;
			}
			idx = idx * 2 + 1;
			if point[dim] >= corner[dim] + size {
				idx += 1;
				corner[dim] += size;
			}
			dim = (dim + 1) % 3;
		}
	}

	pub fn update(
		&self,
		state: &render::State,
		point_clouds: &Mutex<HashMap<usize, (render::PointCloud, render::PointCloudProperty)>>,
		idx: usize,
	) {
		let Some(mut node) = self.nodes.get_mut(&idx) else {
			return;
		};
		match node.value_mut() {
			Node::Branch {} => {
				drop(node);
			},
			Node::Leaf { points, property, dirty } => {
				if dirty.not() {
					return;
				}
				*dirty = false;
				let point_cloud = render::PointCloud::new(state, points);
				let property = render::PointCloudProperty::new(state, property);
				drop(node);
				point_clouds
					.lock()
					.unwrap()
					.insert(idx, (point_cloud, property));
			},
		}
	}

	pub fn render<'a>(
		&self,
		point_cloud_pass: &mut render::PointCloudPass<'a>,
		point_clouds: &'a HashMap<usize, (render::PointCloud, render::PointCloudProperty)>,
	) {
		self.render_with(point_cloud_pass, point_clouds, 0);
	}

	fn render_with<'a>(
		&self,
		point_cloud_pass: &mut render::PointCloudPass<'a>,
		point_clouds: &'a HashMap<usize, (render::PointCloud, render::PointCloudProperty)>,
		mut idx: usize,
	) {
		let Some(mut node) = self.nodes.get_mut(&idx) else {
			return;
		};
		match node.value_mut() {
			Node::Branch { .. } => {
				drop(node);
				idx = idx * 2 + 1;
				for i in 0..2 {
					self.render_with(point_cloud_pass, point_clouds, idx + i)
				}
			},
			Node::Leaf { .. } => {
				if let Some((point_cloud, property)) = point_clouds.get(&idx) {
					point_cloud.render(point_cloud_pass, property);
				}
				drop(node);
			},
		}
	}
}
