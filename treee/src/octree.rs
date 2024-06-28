use std::{
	collections::HashMap,
	ops::{Not, Range},
	sync::Mutex,
};

use dashmap::DashMap;
use nalgebra as na;

pub const MAX_POINTS: usize = 1 << 16;

#[derive(Debug)]
pub struct Octree {
	corner: na::Point3<f32>,
	size: f32,
	nodes: DashMap<usize, Node>,

	pub points_min: na::Point3<f32>,
	pub points_max: na::Point3<f32>,
}

#[derive(Debug)]
enum Node {
	Branch {},
	Leaf {
		points: Vec<na::Point3<f32>>,
		dirty: bool,
	},
}

impl Octree {
	pub fn new(min: na::Point3<f32>, max: na::Point3<f32>) -> Self {
		let diff = max - min;
		let size = diff.x.max(diff.y).max(diff.z);
		Self {
			corner: min,
			size,
			nodes: DashMap::new(),
			points_min: min,
			points_max: max,
		}
	}

	pub fn corner(&self) -> na::Point3<f32> {
		self.corner
	}

	pub fn size(&self) -> f32 {
		self.size
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
		let mut entry = self
			.nodes
			.entry(idx)
			.or_insert_with(|| Node::Leaf { points: Vec::new(), dirty: false });
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
			Node::Leaf { points, dirty } => {
				if points.len() + new_points.len() < MAX_POINTS {
					if dirty.not() {
						*dirty = true;
						changed(idx);
					}
					points.extend_from_slice(new_points);
					return;
				}
				let mut points = std::mem::take(points);
				*node = Node::Branch {};
				changed(idx);
				drop(entry);

				self.insert_with(&mut points, corner, size, idx, dim, changed);
				self.insert_with(new_points, corner, size, idx, dim, changed);
			},
		}
	}

	pub fn update(&self, state: &render::State, point_clouds: &Mutex<HashMap<usize, render::PointCloud>>, idx: usize) {
		let Some(mut node) = self.nodes.get_mut(&idx) else {
			return;
		};
		match node.value_mut() {
			Node::Branch {} => {
				drop(node);
				point_clouds.lock().unwrap().remove(&idx);
			},
			Node::Leaf { points, dirty } => {
				assert!(*dirty);
				*dirty = false;
				let point_cloud = render::PointCloud::new(state, points);
				drop(node);
				point_clouds.lock().unwrap().insert(idx, point_cloud);
			},
		}
	}

	pub fn render<'a>(
		&self,
		point_cloud_pass: &mut render::PointCloudPass<'a>,
		point_clouds: &'a HashMap<usize, render::PointCloud>,
		property: &'a render::PointCloudProperty,
	) {
		self.render_with(point_cloud_pass, point_clouds, property, 0);
	}

	fn render_with<'a>(
		&self,
		point_cloud_pass: &mut render::PointCloudPass<'a>,
		point_clouds: &'a HashMap<usize, render::PointCloud>,
		property: &'a render::PointCloudProperty,
		mut idx: usize,
	) {
		let Some(node) = self.nodes.get(&idx) else {
			return;
		};
		match node.value() {
			Node::Branch { .. } => {
				drop(node);
				idx = idx * 2 + 1;
				for i in 0..2 {
					self.render_with(point_cloud_pass, point_clouds, property, idx + i)
				}
			},
			Node::Leaf { .. } => {
				if let Some(point_cloud) = point_clouds.get(&idx) {
					point_cloud.render(point_cloud_pass, property);
				}
				drop(node);
			},
		}
	}

	pub fn get_range(&self, range: Range<f32>) -> Vec<na::Point3<f32>> {
		let mut points = Vec::new();
		self.get_range_with(range, &mut points, self.corner.y, self.size, 0, 0);
		points
	}

	fn get_range_with(
		&self,
		range: Range<f32>,
		res: &mut Vec<na::Point3<f32>>,
		corner_height: f32,
		mut size: f32,
		idx: usize,
		dim: usize,
	) {
		let Some(node) = self.nodes.get(&idx) else {
			return;
		};
		match node.value() {
			Node::Branch { .. } => {
				drop(node);
				if dim == 0 {
					size /= 2.0;
				}
				let dim = (dim + 1) % 3;
				let idx = idx * 2;
				if dim == 2 {
					let sep = corner_height + size;
					if range.start <= sep {
						self.get_range_with(range.clone(), res, corner_height, size, idx + 1, dim);
					}
					if sep <= range.end {
						self.get_range_with(range, res, sep, size, idx + 2, dim);
					}
				} else {
					self.get_range_with(range.clone(), res, corner_height, size, idx + 1, dim);
					self.get_range_with(range, res, corner_height, size, idx + 2, dim);
				}
			},
			Node::Leaf { points, .. } => {
				for &point in points.iter() {
					if range.contains(&point.y) {
						res.push(point);
					}
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
