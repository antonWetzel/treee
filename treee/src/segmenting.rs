use crossbeam::atomic::AtomicCell;
use dashmap::DashMap;
use nalgebra as na;
use rand::{seq::SliceRandom, thread_rng};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{
	collections::VecDeque,
	ops::{DerefMut, Not},
	sync::{Arc, Mutex, RwLock},
};
use voronator::delaunator::Point;

use crate::{octree::Octree, program::Event};

pub const DEFAULT_MAX_DISTANCE: f32 = 0.75;

pub struct Segmenting {
	pub shared: Arc<Shared>,
	pub distance: f32,
	pub restart: crossbeam::channel::Sender<f32>,
}

pub struct Shared {
	pub state: Arc<render::State>,
	pub point_clouds: Mutex<Vec<(render::PointCloud, render::PointCloudProperty)>>,
	pub done: AtomicCell<Option<DashMap<usize, Vec<na::Point3<f32>>>>>,
}

impl Segmenting {
	pub fn new(
		octree: Octree,
		state: Arc<render::State>,
		max_distance: f32,
	) -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();

		let (restart_sender, restart_reciever) = crossbeam::channel::unbounded();
		restart_sender.send(max_distance).unwrap();

		let shared = Arc::new(Shared {
			state,
			point_clouds: Mutex::new(Vec::new()),
			done: AtomicCell::new(None),
		});

		{
			let shared = shared.clone();
			std::thread::spawn(move || {
				while let Ok(distance) = restart_reciever.recv() {
					segmentation(distance, &octree, &shared, &sender, &restart_reciever);
				}
			});
		}

		(
			Self {
				shared,
				distance: max_distance,
				restart: restart_sender,
			},
			receiver,
		)
	}

	pub fn ui(&mut self, ui: &mut egui::Ui) -> SegmentingResponse {
		let mut response = SegmentingResponse::None;

		if ui
			.add(egui::Slider::new(&mut self.distance, 0.1..=2.0))
			.changed()
		{
			self.restart.send(self.distance).unwrap();
		};

		if let Some(segments) = self.shared.done.take() {
			if ui.button("Continue").clicked() {
				response = SegmentingResponse::Done(segments);
			} else {
				self.shared.done.store(Some(segments));
			}
		}

		response
	}
}

fn segmentation(
	max_distance: f32,
	octree: &Octree,
	segmenting: &Shared,
	sender: &crossbeam::channel::Sender<Event>,
	reciever: &crossbeam::channel::Receiver<f32>,
) {
	segmenting.point_clouds.lock().unwrap().clear();
	let min = octree.points_min.y - 1.0;
	let size = octree.points_max.y + 2.0 - min;
	let layers = size.ceil() as usize;
	let lookup = render::Lookup::new_png(
		&segmenting.state,
		include_bytes!("../../viewer/assets/grad_turbo.png"),
		(layers) as u32,
	);
	sender.send(Event::Lookup(lookup)).unwrap();

	let slices = {
		let (sender, mut receiver) = crossbeam::channel::bounded(1);
		let mut slices = Vec::with_capacity(layers);
		for i in (0..layers).rev() {
			let (next_sender, next_receiver) = crossbeam::channel::bounded(1);
			let range = (min + i as f32)..(min + (i + 1) as f32);
			slices.push((receiver, range, next_sender));
			receiver = next_receiver;
		}
		sender.send(Vec::new()).unwrap();
		slices
	};

	let min = Point {
		x: octree.points_min.x as f64,
		y: octree.points_min.z as f64,
	};
	let max = Point {
		x: octree.points_max.x as f64,
		y: octree.points_max.z as f64,
	};

	let segments = DashMap::<usize, Vec<na::Point3<f32>>>::new();

	let cancel = slices
		.into_iter()
		.par_bridge()
		.any(|(c_receiver, range, c_sender)| {
			if reciever.is_empty().not() {
				return true;
			}

			let mut slice = octree.get_range(range);
			let tree_set = TreeSet::new(&mut slice, max_distance);

			let centroids = c_receiver.recv().unwrap();
			let centroids = tree_set.tree_positions(centroids, max_distance);
			let points = centroids
				.iter()
				.map(|centroid| Point {
					x: centroid.center.x as f64,
					y: centroid.center.y as f64,
				})
				.collect::<Vec<_>>();
			_ = c_sender.send(centroids);

			let vor = voronator::VoronoiDiagram::new(&min, &max, &points).unwrap();
			let mut trees = vor
				.cells()
				.iter()
				.map(|cell| cell.points())
				.map(|p| {
					p.iter()
						.map(|p| na::Point2::new(p.x as f32, p.y as f32))
						.collect::<Vec<_>>()
				})
				.map(Tree::from_points)
				.enumerate()
				.collect::<VecDeque<_>>();

			let mut property = Vec::with_capacity(slice.len());
			for &p in slice.iter() {
				let Some((idx, _)) = trees
					.iter_mut()
					.enumerate()
					.find(|(_, (_, tree))| tree.contains(na::Point2::new(p.x, p.z), 0.1))
				else {
					property.push(0);
					continue;
				};
				let elem = trees.remove(idx).unwrap();
				property.push(elem.0 as u32);
				segments.entry(elem.0).or_default().value_mut().push(p);
				// hope next point is in the same segment
				trees.push_front(elem);
			}

			if slice.is_empty().not() {
				let point_cloud = render::PointCloud::new(&segmenting.state, &slice);
				let property = render::PointCloudProperty::new(&segmenting.state, &property);
				let mut point_clouds = segmenting.point_clouds.lock().unwrap();
				point_clouds.push((point_cloud, property));
			}
			false
		});
	if cancel {
		return;
	}
	segmenting.done.store(Some(segments));
}

pub enum SegmentingResponse {
	None,
	Done(DashMap<usize, Vec<na::Point3<f32>>>),
}

#[derive(Debug, Clone)]
pub struct TreeSet {
	trees: Vec<Tree>,
}

#[derive(Debug, Clone)]
pub struct Tree {
	points: Vec<na::Point2<f32>>,
	min: na::Point2<f32>,
	max: na::Point2<f32>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TreeStatistics {
	pub area: f32,
	pub center: na::Point2<f32>,
}

impl Tree {
	pub fn new(p: na::Point2<f32>) -> Self {
		Self {
			points: vec![
				p,
				na::Point2::new(p.x + 0.1, p.y),
				na::Point2::new(p.x, p.y + 0.1),
			],
			min: p,
			max: p + na::vector![0.1, 0.1],
		}
	}

	pub fn from_points(mut points: Vec<na::Point2<f32>>) -> Self {
		match points.len() {
			0 => {
				return Self {
					points,
					min: [f32::MAX, f32::MAX].into(),
					max: [f32::MIN, f32::MIN].into(),
				}
			},
			1 => {
				points.push(points[0] + na::vector![0.1, 0.0]);
				points.push(points[0] + na::vector![0.0, 0.1]);
			},
			2 => {
				let diff = points[1] - points[0];
				points.push(points[0] + na::vector![-diff.y, diff.x].normalize() * 0.1);
			},
			_ => {},
		}
		let mut min = points[0];
		let mut max = points[0];
		for &p in points.iter().skip(1) {
			min = min.coords.zip_map(&p.coords, |a, b| a.min(b)).into();
			max = max.coords.zip_map(&p.coords, |a, b| a.max(b)).into();
		}
		Self { points, min, max }
	}

	pub fn distance(&self, point: na::Point2<f32>, max_distance: f32) -> f32 {
		if self.outside_bounds(point, max_distance) {
			return f32::MAX;
		}
		let mut best = f32::MIN;
		for i in 0..self.points.len() {
			let a = self.points[i];
			let b = self.points[(i + 1) % self.points.len()];
			let dir = b - a;
			let out = na::vector![dir.y, -dir.x].normalize();
			let diff = point - a;
			let dist = out.dot(&diff);
			if dist > max_distance {
				return f32::MAX;
			}
			best = best.max(dist);
		}
		best
	}

	pub fn outside_bounds(&self, point: na::Point2<f32>, max_distance: f32) -> bool {
		point.x + max_distance < self.min.x
			|| self.max.x + max_distance <= point.x
			|| point.y + max_distance < self.min.y
			|| self.max.y + max_distance <= point.y
	}

	pub fn statistics(&self) -> TreeStatistics {
		let (center, area) = centroid(&self.points);
		TreeStatistics { center, area }
	}

	pub fn contains(&self, point: na::Point2<f32>, max_distance: f32) -> bool {
		if self.outside_bounds(point, max_distance) {
			return false;
		}
		for i in 0..self.points.len() {
			let a = self.points[i];
			let b = self.points[(i + 1) % self.points.len()];
			let dir = b - a;
			let out = na::vector![dir.y, -dir.x].normalize();
			let diff = point - a;
			let dist = out.dot(&diff);
			if dist > max_distance {
				return false;
			}
		}
		true
	}

	pub fn insert(&mut self, point: na::Point2<f32>) {
		fn outside(a: na::Point2<f32>, b: na::Point2<f32>, point: na::Point2<f32>) -> bool {
			let dir = b - a;
			let out = na::vector![dir.y, -dir.x].normalize();
			let diff = point - a;
			let dist = out.dot(&diff);
			dist > 0.0
		}

		let tree = &mut self.points;
		let mut last = outside(tree[tree.len() - 1], tree[0], point);
		let mut start = None;
		let mut end = None;
		for i in 0..tree.len() {
			let a = tree[i];
			let b = tree[(i + 1) % tree.len()];
			let out = outside(a, b, point);
			match (last, out) {
				(false, false) => {},
				(false, true) => start = Some(i),
				(true, true) => {},
				(true, false) => end = Some(i),
			}
			last = out;
		}

		let (Some(start), Some(end)) = (start, end) else {
			return;
		};
		if end < start {
			tree.splice((start + 1)..tree.len(), [point]);
			tree.splice(0..end, []);
		} else {
			tree.splice((start + 1)..end, [point]);
		}

		self.min = self
			.min
			.coords
			.zip_map(&point.coords, |a, b| a.min(b))
			.into();
		self.max = self
			.max
			.coords
			.zip_map(&point.coords, |a, b| a.max(b))
			.into();
	}
}

#[derive(Clone, Copy)]
pub struct Centroid {
	center: na::Point2<f32>,
}

impl TreeSet {
	pub fn new_empty() -> Self {
		Self { trees: Vec::new() }
	}

	pub fn new(points: &mut [na::Point3<f32>], max_distance: f32) -> Self {
		points.shuffle(&mut thread_rng());

		let mut trees = Self::new_empty();
		for &point in points.iter() {
			trees.add_point(point, max_distance);
		}
		trees.filter_trees(max_distance);
		trees
	}

	pub fn add_point(&mut self, point: na::Point3<f32>, max_distance: f32) {
		let mut near = Vec::new();
		let p = na::Point2::new(point.x, point.z);
		for (i, tree) in self.trees.iter().enumerate() {
			let dist = tree.distance(p, max_distance);
			if dist <= 0.0 {
				return;
			}
			if dist <= max_distance {
				near.push(i);
			}
		}
		match near.len() {
			// new
			0 => self.trees.push(Tree::new(p)),

			// insert
			1 => self.trees[near[0]].insert(p),

			// merge
			_ => {
				let target = near[0];
				for other in near[1..].iter().rev().copied() {
					let o = self.trees.remove(other);
					for p in o.points {
						self.trees[target].insert(p);
					}
				}
				self.trees[target].insert(p);
			},
		}
	}

	pub fn filter_trees(&mut self, max_distance: f32) {
		for i in (0..self.trees.len()).rev() {
			let tree = &self.trees[i];
			let (center, area) = centroid(&tree.points);
			if area < (max_distance * max_distance) / 4.0 {
				self.trees.remove(i);
				continue;
			}
			for other in &self.trees[0..i] {
				if other.contains(center, 0.1) {
					self.trees.remove(i);
					break;
				}
			}
		}
	}

	pub fn tree_positions(self, prev: Vec<Centroid>, max_distance: f32) -> Vec<Centroid> {
		let mut res = Vec::with_capacity(prev.len());
		let mut centroids = self
			.trees
			.into_iter()
			.map(|tree| centroid(&tree.points).0)
			.collect::<Vec<_>>();

		for &center in prev.iter() {
			let mut nearest = None;
			let mut nearest_dist = max_distance * 2.0;
			for (idx, &c) in centroids.iter().enumerate() {
				let d = (center.center - c).norm();
				if d < nearest_dist {
					nearest = Some(idx);
					nearest_dist = d;
				}
			}
			if let Some(idx) = nearest {
				let c = centroids.swap_remove(idx);
				res.push(Centroid { center: c });
			} else {
				res.push(center)
			}
		}
		for c in centroids {
			res.push(Centroid { center: c });
		}
		res
	}
}

//https://math.stackexchange.com/questions/90463/how-can-i-calculate-the-centroid-of-polygon
fn centroid(points: &[na::Point2<f32>]) -> (na::Point2<f32>, f32) {
	let mut center = na::vector![0.0, 0.0];
	let mut area = 0.0;

	let a = points[0];
	for i in 1..(points.len() - 1) {
		let b = points[i] - a;
		let c = points[i + 1] - a;
		let t_center = (b + c) / 3.0;
		let t_area = b.x * c.y - b.y * c.x;

		center += t_center * t_area;
		area += t_area;
	}

	(a + center / area, area / 2.0)
}
