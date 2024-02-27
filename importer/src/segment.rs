use math::{Vector, X, Y, Z};
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
	data: CacheEntry<Vector<3, f32>>,
}

impl Segment {
	pub fn points(self) -> Vec<Vector<3, f32>> {
		self.data.read()
	}

	pub fn length(&self) -> usize {
		self.data.length()
	}
}

pub struct Segmenter {
	slices: Vec<CacheIndex<Vector<3, f32>>>,
	min: Vector<3, f32>,
	max: Vector<3, f32>,
	slice_height: f32,
	max_distance: f32,
	min_segment_size: usize,
}

impl Segmenter {
	pub fn new(min: Vector<3, f32>, max: Vector<3, f32>, cache: &mut Cache, settings: &Settings) -> Self {
		let slice_count = ((max[Y] - min[Y]) / settings.segmenting_slice_width) as usize + 1;
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

	pub fn add_point(&mut self, point: Vector<3, f32>, cache: &mut Cache) {
		let slice = ((self.max[Y] - point[Y]) / self.slice_height) as usize;
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
			x: self.min[X] as f64,
			y: self.min[Z] as f64,
		};
		let max = Point {
			x: self.max[X] as f64,
			y: self.max[Z] as f64,
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
										size[X] * 10.0,
										size[Z] * 10.0,
										size[X] * 10.0,
										size[Z] * 10.0
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
						let mut centroids = c_reciever.recv().unwrap();
						tree_set.tree_positions(&mut centroids, self.max_distance);
						let points = centroids
							.iter()
							.map(|centroid| Point {
								x: centroid.center[X] as f64,
								y: centroid.center[Y] as f64,
							})
							.collect::<Vec<_>>();

						#[cfg(feature = "segmentation-svgs")]
						for centroid in &centroids {
							svg.write_all(
								format!(
									"  <circle cx=\"{}\" cy=\"{}\" r=\"10\" />\n",
									(centroid.center[X] - self.min[X]) * 10.0,
									(centroid.center[Y] - self.min[Z]) * 10.0
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
									.map(|p| Vector::new([p.x as f32, p.y as f32]))
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
								.find(|(_, (_, tree, _))| tree.contains(Vector::new([p[X], p[Z]]), 0.1))
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

#[derive(Debug)]
struct TreeSet {
	trees: Vec<Tree>,
}

#[derive(Debug)]
struct Tree {
	points: Vec<Vector<2, f32>>,
	min: Vector<2, f32>,
	max: Vector<2, f32>,
}

impl Tree {
	pub fn new(p: Vector<2, f32>, max_distance: f32) -> Self {
		Self {
			points: vec![
				p,
				Vector::new([p[X] + 0.1, p[Y]]),
				Vector::new([p[X], p[Y] + 0.1]),
			],
			min: p - Vector::new([max_distance, max_distance]),
			max: p + Vector::new([max_distance + 0.1, max_distance + 0.1]),
		}
	}

	pub fn from_points(mut points: Vec<Vector<2, f32>>, max_distance: f32) -> Self {
		match points.len() {
			0 => {
				return Self {
					points,
					min: [f32::MAX, f32::MAX].into(),
					max: [f32::MIN, f32::MIN].into(),
				}
			},
			1 => {
				points.push(points[0] + [0.1, 0.0].into());
				points.push(points[0] + [0.0, 0.1].into());
			},
			2 => {
				let diff = points[1] - points[0];
				points.push(points[0] + Vector::new([-diff[Y], diff[X]]).normalized() * 0.1);
			},
			_ => {},
		}
		let mut min = points[0];
		let mut max = points[0];
		for &p in points.iter().skip(1) {
			min = min.min(p);
			max = max.max(p);
		}
		Self {
			points,
			min: min - Vector::new([max_distance, max_distance]),
			max: max + Vector::new([max_distance, max_distance]),
		}
	}

	fn distance(&self, point: Vector<2, f32>, max_distance: f32) -> f32 {
		if point[X] < self.min[X] || point[X] >= self.max[X] || point[Y] < self.min[Y] || point[Y] >= self.max[Y] {
			return f32::MAX;
		}
		let mut best = f32::MIN;
		for i in 0..self.points.len() {
			let a = self.points[i];
			let b = self.points[(i + 1) % self.points.len()];
			let dir = b - a;
			let out = Vector::new([dir[Y], -dir[X]]).normalized();
			let diff = point - a;
			let dist = out.dot(diff);
			if dist > max_distance {
				return f32::MAX;
			}
			best = best.max(dist);
		}
		best
	}

	fn contains(&self, point: Vector<2, f32>, max_distance: f32) -> bool {
		if point[X] < self.min[X] || point[X] >= self.max[X] || point[Y] < self.min[Y] || point[Y] >= self.max[Y] {
			return false;
		}
		for i in 0..self.points.len() {
			let a = self.points[i];
			let b = self.points[(i + 1) % self.points.len()];
			let dir = b - a;
			let out = Vector::new([dir[Y], -dir[X]]).normalized();
			let diff = point - a;
			let dist = out.dot(diff);
			if dist > max_distance {
				return false;
			}
		}
		true
	}

	fn insert(&mut self, point: Vector<2, f32>, max_distance: f32) {
		fn outside(a: Vector<2, f32>, b: Vector<2, f32>, point: Vector<2, f32>) -> bool {
			let dir = b - a;
			let out = Vector::new([dir[Y], -dir[X]]).normalized();
			let diff = point - a;
			let dist = out.dot(diff);
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
			.min(point - Vector::new([max_distance, max_distance]));
		self.max = self
			.max
			.max(point + Vector::new([max_distance, max_distance]));
	}

	// pub fn intersections(&self, trees: &[Tree]) -> Vec<usize> {
	// 	let mut res = Vec::new();
	// 	for (idx, tree) in trees.iter().enumerate() {
	// 		if self.max[X] < tree.min[X]
	// 			|| tree.max[X] < self.min[X]
	// 			|| self.max[Y] < tree.min[Y]
	// 			|| tree.max[Y] < self.min[Y]
	// 		{
	// 			continue;
	// 		}
	// 		let seperated = (0..self.points.len()).any(|i| {
	// 			let a = self.points[i];
	// 			let b = self.points[(i + 1) % self.points.len()];
	// 			let dir = b - a;
	// 			let out = Vector::new([dir[Y], -dir[X]]).normalized();
	// 			tree.points.iter().all(|&p| {
	// 				let diff = p - a;
	// 				diff.dot(out) >= 0.0
	// 			})
	// 		});
	// 		if seperated.not() {
	// 			res.push(idx);
	// 		}
	// 	}
	// 	res
	// }

	#[cfg(feature = "segmentation-svgs")]
	pub fn save_svg(&self, file: &mut std::fs::File, min: Vector<3, f32>, fill: bool) {
		file.write_all(b"  <polygon points=\"").unwrap();
		for &point in &self.points {
			file.write_all(
				format!(
					"{},{} ",
					(point[X] - min[X]) * 10.0,
					(point[Y] - min[Z]) * 10.0
				)
				.as_bytes(),
			)
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

struct Centroid {
	center: Vector<2, f32>,
}

impl TreeSet {
	pub fn new(points: &[Vector<3, f32>], max_distance: f32) -> Self {
		let mut trees = Vec::<Tree>::new();
		'iter_points: for &point in points {
			let mut near = Vec::new();
			let p = Vector::new([point[X], point[Z]]);
			for (i, tree) in trees.iter().enumerate() {
				let dist = tree.distance(p, max_distance);
				if dist <= 0.0 {
					continue 'iter_points;
				}
				if dist <= max_distance {
					near.push(i);
				}
			}
			match near.len() {
				// new
				0 => trees.push(Tree::new(p, max_distance)),

				// insert
				1 => trees[near[0]].insert(p, max_distance),

				// merge
				_ => {
					let target = near[0];
					for other in near[1..].iter().rev().copied() {
						let o = trees.remove(other);
						for p in o.points {
							trees[target].insert(p, max_distance);
						}
					}
					trees[target].insert(p, max_distance);
				},
			}
		}

		for i in (0..trees.len()).rev() {
			let tree = &trees[i];
			let (center, area) = centroid(&tree.points);
			if area < (max_distance * max_distance) / 4.0 {
				trees.remove(i);
				continue;
			}
			for other in &trees[0..i] {
				if other.contains(center, 0.1) {
					trees.remove(i);
					break;
				}
			}
		}

		Self { trees }
	}

	pub fn tree_positions(&self, prev: &mut Vec<Centroid>, max_distance: f32) {
		// let mut res = Vec::new();
		for tree in &self.trees {
			let mut contains = Vec::new();
			for centroid in prev.iter_mut() {
				if tree.contains(centroid.center, 2.0 * max_distance) {
					contains.push(centroid);
				}
			}
			match contains.len() {
				0 => {
					prev.push(Centroid { center: centroid(&tree.points).0 });
				},
				1 => {
					contains[0].center = centroid(&tree.points).0;
				},
				_ => {},
			}
		}
	}
}

//https://math.stackexchange.com/questions/90463/how-can-i-calculate-the-centroid-of-polygon
fn centroid(points: &[Vector<2, f32>]) -> (Vector<2, f32>, f32) {
	let mut center = Vector::new([0.0, 0.0]);
	let mut area = 0.0;

	let a = points[0];
	for i in 1..(points.len() - 1) {
		let b = points[i] - a;
		let c = points[i + 1] - a;
		let t_center = (b + c) / 3.0;
		let t_area = (b[X] * c[Y] - b[Y] * c[X]) / 2.0;

		center += t_center * t_area;
		area += t_area;
	}

	(a + center / area, area)
}
