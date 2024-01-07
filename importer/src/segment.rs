use std::collections::{ HashMap, hash_map::Entry };

use math::{ Vector, X, Y, Z };

use crate::cache::{ Cache, CacheEntry, CacheIndex };


const SLICE_HEIGHT: f32 = 0.5;
const FIELD_SIZE: f32 = 0.5;
const MAX_DIST: usize = 6;
const ITERATIONS: usize = 2;


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
	cache: Cache<Vector<3, f32>>,
	slices: Vec<CacheIndex>,
	min: Vector<3, f32>,
	max: Vector<3, f32>,
}


impl Segmenter {
	pub fn new(min: Vector<3, f32>, max: Vector<3, f32>) -> Self {
		let slice_count = ((max[Y] - min[Y]) / SLICE_HEIGHT) as usize + 1;
		let mut cache = Cache::new(100_000_000); // 1.2 GB
		let slices = (0..slice_count).map(|_| cache.new_entry()).collect();
		Self { slices, min, max, cache }
	}


	pub fn add_point(&mut self, point: Vector<3, f32>) {
		let slice = ((point[Y] - self.min[Y]) / SLICE_HEIGHT) as usize;
		self.cache.add_entry(&self.slices[slice], point);
	}


	pub fn segments(mut self) -> Vec<Segment> {
		let mut segments = Vec::new();
		let width = ((self.max[X] - self.min[X]) / FIELD_SIZE) as usize + 1;
		let depth = ((self.max[Z] - self.min[Z]) / FIELD_SIZE) as usize + 1;

		let mut cache = Cache::new(100_000_000);
		let mut field = Field::new(self.min, width, depth);
		for slice in self.slices.into_iter().rev() {
			let slice = self.cache.read(slice).read();
			for p in slice {
				let idx = field.index(p);
				match field.get(idx) {
					None => {
						let mut id = segments.len();
						if let Some(new_id) = field.set_seed(idx, id) {
							id = new_id;
						} else {
							segments.push(cache.new_entry());
						}
						cache.add_entry(&segments[id], p);
					}
					Some(id) => {
						cache.add_entry(&segments[id], p);
						field.set(idx, id);
					},
				};
			}
			field.step();
		}

		let mut segments = segments.into_iter().map(|entry| Segment { data: cache.read(entry) }).collect::<Vec<_>>();
		segments.sort_by(|a, b| b.data.active().cmp(&a.data.active()));
		segments
	}
}


struct Field {
	data: Vec<(usize, usize)>,
	width: usize,
	depth: usize,
	min: Vector<3, f32>,
}


impl Field {
	pub fn new(min: Vector<3, f32>, width: usize, depth: usize) -> Self {
		Self { width, depth, data: vec![(usize::MAX, usize::MAX); width * depth], min }
	}


	pub fn index(&self, p: Vector<3, f32>) -> usize {
		let x = ((p[X] - self.min[X]) / FIELD_SIZE) as usize;
		let z = ((p[Z] - self.min[Z]) / FIELD_SIZE) as usize;
		x + z * self.width
	}


	pub fn get(&self, index: usize) -> Option<usize> {
		let (_, id) = self.data[index];
		(id < usize::MAX).then_some(id)
	}


	pub fn set(&mut self, index: usize, id: usize) {
		self.data[index] = (0, id);
	}


	pub fn set_seed(&mut self, index: usize, id: usize) -> Option<usize> {
		let x = index % self.width;
		let z = index / self.width;
		let mut found = HashMap::<usize, usize>::new();
		for d_z in z.saturating_sub(5)..(z + 3).min(self.depth) {
			for d_x in x.saturating_sub(2)..(x + 3).min(self.width) {
				let index = d_x + d_z * self.width;
				let (_, idx) = self.data[d_x + d_z * self.width];
				if idx == usize::MAX {
					continue;
				}
				match found.entry(idx) {
					Entry::Occupied(mut e) => {
						*e.get_mut() += 1;
					},
					Entry::Vacant(e) => {
						e.insert(1);
					}
				}
				if self.data[index].1 == usize::MAX {
					self.data[index] = (0, id);
				}
			}
		}
		let res = found.iter().max_by_key(|(_, &v)| v).map(|(&idx, _)| idx).unwrap_or(id);
		for d_z in z.saturating_sub(2)..(z + 3).min(self.depth) {
			for d_x in x.saturating_sub(2)..(x + 3).min(self.width) {
				let index = d_x + d_z * self.width;
				self.data[index] = (0, res);
			}
		}
		(res != id).then_some(res)
	}


	pub fn step(&mut self) {
		for _ in 0..ITERATIONS {
			let mut new = Vec::with_capacity(self.data.len());
			for z in 0..self.depth {
				for x in 0..self.width {
					let mut found = HashMap::<usize, (usize, usize)>::new();
					for d_z in z.saturating_sub(1)..(z + 2).min(self.depth) {
						for d_x in x.saturating_sub(1)..(x + 2).min(self.width) {
							let (v, idx) = self.data[d_x + d_z * self.width];

							if idx == usize::MAX {
								continue;
							}
							let v = v + 1 + x.abs_diff(d_x) + z.abs_diff(d_z);
							if v > MAX_DIST {
								continue;
							}
							match found.entry(idx) {
								Entry::Occupied(mut e) => {
									let e = e.get_mut();
									e.0 = e.0.min(v);
									e.1 += 1;
								},
								Entry::Vacant(e) => {
									e.insert((v, 1));
								}
							}
						}
					}
					let res = found.iter().max_by_key(|(_, (_, v))| v).map(|(&idx, &(d, _))| (d, idx)).unwrap_or((usize::MAX, usize::MAX));
					new.push(res);
				}
			}
			self.data = new;
		}
	}
}
