use std::{
	cmp::Ordering,
	collections::{HashMap, HashSet},
	ops::Not,
};

use math::{Vector, X, Y, Z};

use crate::cache::{Cache, CacheEntry, CacheIndex};

pub struct Segment {
	data: CacheEntry<Vector<3, f32>>,
}

impl Segment {
	pub fn points(self) -> Vec<Vector<3, f32>> {
		self.data.read()
	}
}

pub struct Segmenter {
	data: HashSet<(usize, usize, usize)>,
	min: Vector<3, f32>,
}

impl Segmenter {
	pub fn new(min: Vector<3, f32>, max: Vector<3, f32>) -> Self {
		Self { data: HashSet::new(), min }
	}

	pub fn add_point(&mut self, point: Vector<3, f32>) {
		let x = ((point[X] - self.min[X]) / 0.05) as usize;
		let y = ((point[Y] - self.min[Y]) / 0.05) as usize;
		let z = ((point[Z] - self.min[Z]) / 0.05) as usize;
		self.data.insert((x, y, z));
	}

	pub fn segments(self) -> Segments {
		let mut cache = Cache::new(16);

		let mut ground = HashMap::<_, f32>::new();
		for &(x, y, z) in self.data.iter() {
			let x = x / 100;
			let y = y as f32 * 0.05;
			let z = z / 100;
			if let Some(ground) = ground.get_mut(&(x, z)) {
				*ground = (*ground).min(y);
			} else {
				ground.insert((x, z), y);
			}
		}

		Segments {
			segments: vec![cache.new_entry(), cache.new_entry()],
			cache,
			min: self.min,
			// data: self.data,
			ground,
		}
	}
}

pub struct Segments {
	cache: Cache<Vector<3, f32>>,
	segments: Vec<CacheIndex>,

	// data: HashSet<(usize, usize, usize)>,
	ground: HashMap<(usize, usize), f32>,
	min: Vector<3, f32>,
}

impl Segments {
	pub fn add_point(&mut self, point: Vector<3, f32>) {
		let x = ((point[X] - self.min[X]) / 0.05) as usize;
		// let y = ((point[Y] - self.min[Y]) / 0.05) as usize;
		let z = ((point[Z] - self.min[Z]) / 0.05) as usize;

		let ground = *self.ground.get(&(x / 100, z / 100)).unwrap();

		let distance = (point[Y] - self.min[Y]) - ground;
		let idx = (distance / 1.0) as usize;
		while idx >= self.segments.len() {
			self.segments.push(self.cache.new_entry());
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
