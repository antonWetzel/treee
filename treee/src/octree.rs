use std::{collections::HashMap, ops::Not, sync::Mutex};

use crossbeam::deque::Injector;
use dashmap::DashMap;
use nalgebra as na;

use crate::task::Task;

const MAX_POINTS: usize = 1 << 14;

pub struct Octree {
	corner: na::Point3<f32>,
	size: f32,
	nodes: DashMap<usize, Node>,
}

enum Node {
	Branch {},
	Leaf {
		points: Vec<project::Point>,
		property: Vec<u32>,
		dirty: bool,
	},
}

impl Octree {
	pub fn new(corner: na::Point3<f32>, size: f32) -> Self {
		Self { corner, size, nodes: DashMap::new() }
	}

	pub fn insert(&self, point: na::Point3<f32>, chunk: usize, injector: &Injector<Task>) {
		let idx = 0;
		let corner = self.corner;
		let size = self.size;
		let point = project::Point { position: point, size: 0.1 };
		self.insert_with(point, chunk as u32, corner, size, idx, injector);
	}

	fn insert_with(
		&self,
		point: project::Point,
		chunk: u32,
		mut corner: na::Point3<f32>,
		mut size: f32,
		mut idx: usize,
		injector: &Injector<Task>,
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
						injector.push(Task::Update(idx));
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
						self.insert_with(point, chunk, corner, size, idx, injector);
					}
					self.insert_with(point, chunk, corner, size, idx, injector);
				},
			}

			size /= 2.0;
			idx = idx * 8 + 1;
			for i in 0..3 {
				if point.position[i] >= corner[i] + size {
					idx += 1 << i;
					corner[i] += size;
				}
			}
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
				idx = idx * 8 + 1;
				for i in 0..8 {
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
