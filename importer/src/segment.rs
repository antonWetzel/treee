use std::{
	cmp::Ordering,
	collections::{HashMap, HashSet},
	num::NonZeroUsize,
	ops::Not,
};

use math::{Vector, X, Y, Z};
use rayon::prelude::*;

use crate::{
	cache::{Cache, CacheEntry, CacheIndex},
	quad_tree::QuadTree,
};

pub struct Segment {
	data: CacheEntry<Vector<3, f32>>,
}

impl Segment {
	pub fn points(self) -> Vec<Vector<3, f32>> {
		self.data.read()
	}
}

fn discretize(point: Vector<3, f32>, min: Vector<3, f32>) -> Vector<3, usize> {
	let x = ((point[X] - min[X]) / 0.05) as usize;
	let y = ((point[Y] - min[Y]) / 0.05) as usize;
	let z = ((point[Z] - min[Z]) / 0.05) as usize;
	[x, y, z].into()
}

pub struct Segmenter {
	data: HashSet<Vector<3, usize>>,
	min: Vector<3, f32>,
}

impl Segmenter {
	pub fn new(min: Vector<3, f32>) -> Self {
		Self { data: HashSet::new(), min }
	}

	pub fn add_point(&mut self, point: Vector<3, f32>) {
		self.data.insert(discretize(point, self.min));
	}

	pub fn segments(self) -> Segments {
		let mut ground = HashMap::<_, usize>::new();
		let mut min = Vector::new([usize::MAX, usize::MAX]);
		let mut max = Vector::new([0, 0]);

		for [x, y, z] in self.data.iter().copied().map(Vector::data) {
			if x < min[X] {
				min[X] = x;
			}
			if y < min[Y] {
				min[Y] = y;
			}
			if x > max[X] {
				max[X] = x;
			}
			if y > max[Y] {
				max[Y] = y;
			}

			let x = x / 60;
			let z = z / 60;
			if let Some(ground) = ground.get_mut(&(x, z)) {
				*ground = (*ground).min(y);
			} else {
				ground.insert((x, z), y);
			}
		}

		let diff = max - min;

		let mut indices = HashMap::new();
		let mut current_index = NonZeroUsize::new(1).unwrap();
		let mut ground_indices = HashMap::new();
		let mut slices = Vec::new();
		for [x, y, z] in self.data.into_iter().map(Vector::data) {
			let ground = *ground.get(&(x / 60, z / 60)).unwrap();

			let distance = y - ground;
			if distance < 40 {
				let index = ground_indices
					.get(&(x / 100, z / 100))
					.copied()
					.unwrap_or_else(|| {
						current_index = current_index.saturating_add(1);
						ground_indices.insert((x / 100, z / 100), current_index);
						current_index
					});
				indices.insert((x, y, z), index);
			} else {
				let idx = y;
				if idx >= slices.len() {
					slices.resize_with(idx + 1, || Vec::new());
				}
				slices[idx].push((x, z));
			}
		}
		drop(ground_indices);

		let mut lookup = QuadTree::<NonZeroUsize>::new(min, diff[X].max(diff[Y]));
		let mut segments = Vec::new();
		for (y, slice) in slices.into_iter().enumerate().rev() {
			slice
				.par_iter()
				.map(|&(x, z)| lookup.get([x, y, z].into(), 60))
				.collect_into_vec(&mut segments);

			for ((x, z), &index) in slice.into_iter().zip(&segments) {
				let index = if let Some(index) = index {
					index
				} else {
					current_index = current_index.saturating_add(1);
					current_index
				};
				lookup.set([x, y, z].into(), index);
				indices.insert((x, y, z), index);
			}
		}

		Segments {
			segments: Vec::new(),
			cache: Cache::new(256),
			min: self.min,
			indices,
		}
	}
}

pub struct Segments {
	cache: Cache<Vector<3, f32>>,
	segments: Vec<CacheIndex>,

	indices: HashMap<(usize, usize, usize), NonZeroUsize>,
	min: Vector<3, f32>,
}

impl Segments {
	pub fn add_point(&mut self, point: Vector<3, f32>) {
		let x = ((point[X] - self.min[X]) / 0.05) as usize;
		let y = ((point[Y] - self.min[Y]) / 0.05) as usize;
		let z = ((point[Z] - self.min[Z]) / 0.05) as usize;

		let idx = self.indices.get(&(x, y, z)).copied().unwrap().get();

		if idx >= self.segments.len() {
			self.segments
				.resize_with(idx + 1, || self.cache.new_entry());
		}

		self.cache.add_point(&self.segments[idx], point);
	}

	pub fn segments(mut self) -> Vec<Segment> {
		let mut res = self
			.segments
			.into_iter()
			.map(|index| self.cache.read(index))
			.filter(|d| d.is_empty().not())
			.map(|d| Segment { data: d })
			.collect::<Vec<_>>();

		res.sort_by(|a, b| match (a.data.active(), b.data.active()) {
			(true, true) | (false, false) => Ordering::Equal,
			(true, false) => Ordering::Less,
			(false, true) => Ordering::Greater,
		});

		res
	}
}
