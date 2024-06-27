use std::{collections::HashMap, ops::Not, sync::Mutex};

use dashmap::DashMap;
use nalgebra as na;

const MAX_POINTS: usize = 1 << 14;

pub struct Octree {
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
	pub fn new(corner: na::Point3<f32>, size: f32) -> Self {
		Self { corner, size, nodes: DashMap::new() }
	}

	pub fn insert(&self, mut points: Vec<na::Point3<f32>>, mut changed: impl FnMut(usize)) {
		let idx = 0;
		let corner = self.corner;
		let size = self.size;
		self.insert_with(&mut points, corner, size, idx, 0, &mut changed);
	}

	fn insert_with(
		&self,
		new_points: &mut [na::Point3<f32>],
		corner: na::Point3<f32>,
		mut size: f32,
		idx: usize,
		dim: usize,
		changed: &mut impl FnMut(usize),
	) {
		let mut entry = self.nodes.entry(idx).or_insert_with(|| Node::Leaf {
			points: Vec::new(),
			property: Vec::new(),
			dirty: false,
		});
		let node = entry.value_mut();
		match node {
			Node::Branch {} => {
				drop(entry);
				if dim == 0 {
					size /= 2.0;
				}
				let sep = partition_points(new_points, corner[dim] + size, dim);
				let (left, right) = new_points.split_at_mut(sep);
				let mut right_corner = corner;
				right_corner[dim] += size;
				let dim = (dim + 1) % 3;
				if left.is_empty().not() {
					self.insert_with(left, corner, size, idx * 2 + 1, dim, changed);
				}
				if right.is_empty().not() {
					self.insert_with(right, right_corner, size, idx * 2 + 2, dim, changed);
				}
			},
			Node::Leaf { points, property, dirty } => {
				if points.len() + new_points.len() < MAX_POINTS {
					if dirty.not() {
						*dirty = true;
						changed(idx);
					}
					points.extend_from_slice(new_points);
					for _ in 0..new_points.len() {
						property.push(0);
					}
					return;
				}
				let mut points = std::mem::take(points);
				*node = Node::Branch {};
				drop(entry);

				self.insert_with(&mut points, corner, size, idx, dim, changed);
				self.insert_with(new_points, corner, size, idx, dim, changed);
			},
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
				assert!(*dirty);
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

fn partition_points(points: &mut [na::Point3<f32>], sep: f32, dim: usize) -> usize {
	if points.is_empty() {
		return 0;
	}
	let mut start = 0;
	let mut end = points.len() - 1;

	while start < end {
		if points[start][dim] >= sep {
			points.swap(start, end);
			end -= 1;
		} else {
			start += 1;
		}
	}
	start
}
