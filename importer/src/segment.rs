use nalgebra as na;
use rayon::prelude::*;
use voronator::delaunator::Point;

#[cfg(feature = "segmentation-svgs")]
use std::io::Write;

use crate::{
	cache::{Cache, CacheEntry, CacheIndex},
	progress::Progress,
	Settings, Statistics,
};

pub struct Segment {
	data: CacheEntry<na::Point3<f32>>,
}

impl Segment {
	pub fn points(self) -> Vec<na::Point3<f32>> {
		self.data.read()
	}

	pub fn length(&self) -> usize {
		self.data.length()
	}

	pub fn new(data: CacheEntry<na::Point3<f32>>) -> Self {
		Self { data }
	}
}

pub struct Segmenter {
	slices: Vec<CacheIndex<na::Point3<f32>>>,
	min: na::Point3<f32>,
	max: na::Point3<f32>,
	slice_height: f32,
	max_distance: f32,
	min_segment_size: usize,
}

impl Segmenter {
	pub fn new(min: na::Point3<f32>, max: na::Point3<f32>, cache: &mut Cache, settings: &Settings) -> Self {
		let slice_count = ((max.y - min.y) / settings.segmenting_slice_width) as usize + 1;
		let slices = (0..slice_count).map(|_| cache.new_entry()).collect();
		Self {
			slices,
			min,
			max,
			slice_height: settings.segmenting_slice_width,
			max_distance: settings.segmenting_max_distance,
			min_segment_size: settings.min_segment_size,
		}
	}

	pub fn add_point(&mut self, point: na::Point3<f32>, cache: &mut Cache) {
		let slice = ((self.max.y - point.y) / self.slice_height) as usize;
		cache.add_value(&self.slices[slice], point);
	}

	pub fn segments(self, statistics: &mut Statistics, cache: &mut Cache) -> Vec<Segment> {
		let total = self
			.slices
			.iter()
			.map(|slice| cache.size(slice))
			.sum::<usize>();
		let mut progress = Progress::new("Segmenting", total);

		let min = Point {
			x: self.min.x as f64,
			y: self.min.z as f64,
		};
		let max = Point {
			x: self.max.x as f64,
			y: self.max.z as f64,
		};

		cfg_if::cfg_if! {
			if #[cfg(feature = "segmentation-svgs")] {
				let size = self.max - self.min;
				_ = std::fs::remove_dir_all("./svg");
				std::fs::create_dir_all("./svg").unwrap();
			}
		}

		let (sender, reciever) = crossbeam::channel::bounded(rayon::current_num_threads());

		let (slices, c_reciever) = {
			let (sender, mut reciever) = crossbeam::channel::bounded(1);
			let mut slices = Vec::with_capacity(self.slices.len());
			for slice in self.slices {
				let (next_sender, next_reciever) = crossbeam::channel::bounded(1);
				slices.push((reciever, cache.read(slice), next_sender));
				reciever = next_reciever;
			}
			sender.send(Vec::new()).unwrap();
			(slices, reciever)
		};

		let (_, segments) = rayon::join(
			move || {
				slices
					.into_iter()
					.enumerate()
					.par_bridge()
					// .into_par_iter()
					.for_each(|(_index, (c_reciever, slice, c_sender))| {
						cfg_if::cfg_if! {
							if #[cfg(feature = "segmentation-svgs")] {
								let mut svg = std::fs::File::create(format!("./svg/test_{}.svg", _index)).unwrap();
								svg.write_all(
									format!(
										"<svg viewbox=\"0 0 {} {}\" xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" >\n",
										size.x * 10.0,
										size.z * 10.0,
										size.x * 10.0,
										size.z * 10.0
									)
									.as_bytes(),
								)
								.unwrap();
							}
						}
						let slice = slice.read();
						let tree_set = TreeSet::new(&slice, self.max_distance);

						#[cfg(feature = "segmentation-svgs")]
						for tree in &tree_set.trees {
							tree.save_svg(&mut svg, self.min, true);
						}

						// println!("pre: {} {:?}", index, std::time::Instant::now());
						let centroids = c_reciever.recv().unwrap();
						let centroids = tree_set.tree_positions(centroids, self.max_distance);
						let points = centroids
							.iter()
							.map(|centroid| Point {
								x: centroid.center.x as f64,
								y: centroid.center.y as f64,
							})
							.collect::<Vec<_>>();

						#[cfg(feature = "segmentation-svgs")]
						for centroid in &centroids {
							svg.write_all(
								format!(
									"  <circle cx=\"{}\" cy=\"{}\" r=\"10\" />\n",
									(centroid.center.x - self.min.x) * 10.0,
									(centroid.center.y - self.min.z) * 10.0
								)
								.as_bytes(),
							)
							.unwrap();
						}
						c_sender.send(centroids).unwrap();

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
							.map(|p| Tree::from_points(p, 0.1))
							.enumerate()
							.map(|(index, tree)| (index, tree, Vec::new()))
							.collect::<Vec<_>>();

						cfg_if::cfg_if! {
							if #[cfg(feature = "segmentation-svgs")] {
								for (_, tree, _) in &trees {
									tree.save_svg(&mut svg, self.min, false);
								}
								svg.write_all(b"</svg>").unwrap();
							}
						}

						for p in slice {
							let Some((idx, _)) = trees
								.iter_mut()
								.enumerate()
								.find(|(_, (_, tree, _))| tree.contains(na::Point2::new(p.x, p.z), 0.1))
							else {
								continue;
							};
							trees[idx].2.push(p);
							// hope next point is in the same segment
							trees.swap(0, idx);
						}
						sender.send(trees).unwrap();
					});
				drop(sender);
			},
			|| {
				let mut segments = Vec::new();
				for trees in reciever {
					for (id, _, points) in trees {
						let l = points.len();
						if id >= segments.len() {
							segments.resize_with(id + 1, || cache.new_entry());
						}
						cache.add_chunk(&segments[id], points);
						progress.step_by(l);
					}
				}
				drop(c_reciever);
				segments
			},
		);

		statistics.times.segment = progress.finish();

		let mut segments = segments
			.into_iter()
			.map(|entry| Segment { data: cache.read(entry) })
			.filter(|entry| entry.length() >= self.min_segment_size)
			.collect::<Vec<_>>();
		segments.sort_by(|a, b| b.data.active().cmp(&a.data.active()));
		segments
	}
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
	pub fn new(p: na::Point2<f32>, max_distance: f32) -> Self {
		Self {
			points: vec![
				p,
				na::Point2::new(p.x + 0.1, p.y),
				na::Point2::new(p.x, p.y + 0.1),
			],
			min: p - na::vector![max_distance, max_distance],
			max: p + na::vector![max_distance + 0.1, max_distance + 0.1],
		}
	}

	pub fn from_points(mut points: Vec<na::Point2<f32>>, max_distance: f32) -> Self {
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
		Self {
			points,
			min: min - na::vector![max_distance, max_distance],
			max: max + na::vector![max_distance, max_distance],
		}
	}

	pub fn distance(&self, point: na::Point2<f32>, max_distance: f32) -> f32 {
		if point.x < self.min.x || point.x >= self.max.x || point.y < self.min.y || point.y >= self.max.y {
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

	pub fn statistics(&self) -> TreeStatistics {
		let (center, area) = centroid(&self.points);
		// let circle_radius = (area / std::f32::consts::PI).sqrt();

		TreeStatistics { center, area }
	}

	pub fn contains(&self, point: na::Point2<f32>, max_distance: f32) -> bool {
		if point.x < self.min.x || point.x >= self.max.x || point.y < self.min.y || point.y >= self.max.y {
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

	pub fn insert(&mut self, point: na::Point2<f32>, max_distance: f32) {
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
			.zip_map(
				&(point - na::vector![max_distance, max_distance]).coords,
				|a, b| a.min(b),
			)
			.into();
		self.max = self
			.max
			.coords
			.zip_map(
				&(point + na::vector![max_distance, max_distance]).coords,
				|a, b| a.max(b),
			)
			.into();
	}

	#[cfg(feature = "segmentation-svgs")]
	pub fn save_svg(&self, file: &mut std::fs::File, min: na::Point3<f32>, fill: bool) {
		file.write_all(b"  <polygon points=\"").unwrap();
		for &point in &self.points {
			file.write_all(format!("{},{} ", (point.x - min.x) * 10.0, (point.y - min.z) * 10.0).as_bytes())
				.unwrap();
		}
		if fill {
			file.write_all(
				format!(
					"\" fill=\"rgb({}, {}, {})\"/>\n",
					rand::random::<u8>(),
					rand::random::<u8>(),
					rand::random::<u8>(),
				)
				.as_bytes(),
			)
			.unwrap();
		} else {
			file.write_all("\" stroke=\"black\" fill=\"none\" />\n".as_bytes())
				.unwrap();
		}
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

	pub fn new(points: &[na::Point3<f32>], max_distance: f32) -> Self {
		let mut trees = Self::new_empty();
		for &point in points {
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
			0 => self.trees.push(Tree::new(p, max_distance)),

			// insert
			1 => self.trees[near[0]].insert(p, max_distance),

			// merge
			_ => {
				let target = near[0];
				for other in near[1..].iter().rev().copied() {
					let o = self.trees.remove(other);
					for p in o.points {
						self.trees[target].insert(p, max_distance);
					}
				}
				self.trees[target].insert(p, max_distance);
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
		let t_area = (b.x * c.y - b.y * c.x) / 2.0;

		center += t_center * t_area;
		area += t_area;
	}

	(a + center / area, area)
}
