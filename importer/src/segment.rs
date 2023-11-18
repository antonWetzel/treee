use std::{
	cmp::Ordering,
	collections::{HashMap, HashSet},
	num::{NonZeroU32, NonZeroUsize},
	ops::Not,
};

use math::{Vector, X, Y, Z};

use crate::cache::{Cache, CacheEntry, CacheIndex};

use self::quad_tree::QuadTree;

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
		let mut ground = HashMap::<_, usize>::new();
		let mut min = Vector::new([usize::MAX, usize::MAX]);
		let mut max = Vector::new([0, 0]);

		println!("start segmenting");
		for &(x, y, z) in self.data.iter() {
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

			let x = x / 100;
			let z = z / 100;
			if let Some(ground) = ground.get_mut(&(x, z)) {
				*ground = (*ground).min(y);
			} else {
				ground.insert((x, z), y);
			}
		}
		println!("ground");

		let diff = max - min;
		let mut lookup = QuadTree::<NonZeroUsize>::new(min, diff[X].max(diff[Y]));

		let mut slices = Vec::new();
		for (x, y, z) in self.data.into_iter() {
			let ground = *ground.get(&(x / 100, z / 100)).unwrap();

			let distance = y - ground;
			let idx = distance / 20;
			while idx >= slices.len() {
				slices.push(HashSet::new());
			}
			slices[idx].insert((x, y, z));
		}
		println!("slices");

		let mut indices = HashMap::new();

		let mut current_index = NonZeroUsize::new(1).unwrap();
		for slice in slices.into_iter().rev() {
			for (x, y, z) in slice {
				let index = if let Some(index) = lookup.get_area(
					Vector::new([
						x.checked_sub(20).unwrap_or_default(),
						z.checked_sub(20).unwrap_or_default(),
					]),
					Vector::new([x + 20, z + 20]),
				) {
					index
				} else {
					current_index = current_index.saturating_add(1);
					current_index
				};
				lookup.set([x, z].into(), index);
				indices.insert((x, y, z), index);
			}
		}
		println!("indices ({})", current_index);

		Segments {
			segments: Vec::new(),
			cache: Cache::new(1024 * 1024),
			min: self.min,
			// data: self.data,
			indices,
		}
	}
}

pub struct Segments {
	cache: Cache<Vector<3, f32>>,
	segments: Vec<CacheIndex>,

	// data: HashSet<(usize, usize, usize)>,
	indices: HashMap<(usize, usize, usize), NonZeroUsize>,
	min: Vector<3, f32>,
}

impl Segments {
	pub fn add_point(&mut self, point: Vector<3, f32>) {
		let x = ((point[X] - self.min[X]) / 0.05) as usize;
		let y = ((point[Y] - self.min[Y]) / 0.05) as usize;
		let z = ((point[Z] - self.min[Z]) / 0.05) as usize;

		let idx = self
			.indices
			.get(&(x, y, z))
			.copied()
			.unwrap_or(NonZeroUsize::new(1).unwrap())
			.get();

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

pub mod quad_tree {
	use std::{collections::HashMap, hash::Hash};

	use math::{Vector, X, Y};

	#[derive(Debug)]
	pub struct QuadTree<T>
	where
		T: Eq + Copy + Hash,
	{
		root: Node<T>,
		min: Vector<2, usize>,
		size: usize,
	}

	impl<T: Eq + Copy + Hash> QuadTree<T> {
		pub fn new(min: Vector<2, usize>, size: usize) -> Self {
			Self { root: Node::Leaf(None), min, size }
		}

		pub fn set(&mut self, position: Vector<2, usize>, value: T) {
			self.root.set(position, value, self.min, self.size);
		}

		pub fn get(&self, position: Vector<2, usize>) -> Option<T> {
			self.root.get(position, self.min, self.size)
		}

		pub fn get_area(&self, min: Vector<2, usize>, max: Vector<2, usize>) -> Option<T> {
			self.root
				.get_area(min, max, self.min, self.size)
				.map(|(value, _)| value)
		}
	}

	#[derive(Debug)]
	enum Node<T>
	where
		T: Eq + Copy + Hash,
	{
		Branch(Box<[Node<T>; 4]>),
		Leaf(Option<T>),
	}

	impl<T: Eq + Copy + Hash> Node<T> {
		pub fn set(
			&mut self,
			position: Vector<2, usize>,
			value: T,
			mut min: Vector<2, usize>,
			mut size: usize,
		) -> bool {
			match self {
				Node::Branch(children) => {
					let mut idx = 0;
					size /= 2;
					if position[X] >= min[X] + size {
						idx += 1;
						min[X] += size;
					}
					if position[Y] >= min[Y] + size {
						idx += 2;
						min[Y] += size;
					}
					if children[idx].set(position, value, min, size) {
						let uniform = children
							.iter()
							.all(|c| matches!(c, &Node::Leaf(Some(other)) if value == other));
						if uniform {
							*self = Node::Leaf(Some(value));
						}
						uniform
					} else {
						false
					}
				},
				Node::Leaf(old) => {
					if size == 1 {
						*old = Some(value);
						return true;
					}
					let mut idx = 0;
					size /= 2;
					if position[X] >= min[X] + size {
						idx += 1;
						min[X] += size;
					}
					if position[Y] >= min[Y] + size {
						idx += 2;
						min[Y] += size;
					}

					let mut children = [
						Node::Leaf(None),
						Node::Leaf(None),
						Node::Leaf(None),
						Node::Leaf(None),
					];
					children[idx].set(position, value, min, size);
					*self = Node::Branch(Box::new(children));
					false
				},
			}
		}

		pub fn get(&self, position: Vector<2, usize>, mut min: Vector<2, usize>, mut size: usize) -> Option<T> {
			match self {
				Node::Branch(children) => {
					let mut idx = 0;
					size /= 2;
					if position[X] >= min[X] + size {
						idx += 1;
						min[X] += size;
					}
					if position[Y] >= min[Y] + size {
						idx += 2;
						min[Y] += size;
					}
					children[idx].get(position, min, size)
				},
				Node::Leaf(value) => *value,
			}
		}

		pub fn get_area(
			&self,
			min: Vector<2, usize>,
			max: Vector<2, usize>,
			corner: Vector<2, usize>,
			mut size: usize,
		) -> Option<(T, usize)> {
			match self {
				Node::Branch(children) => {
					size /= 2;
					let min_x = if min[X] < corner[X] + size { 0 } else { 1 };
					let min_y = if min[Y] < corner[Y] + size { 0 } else { 1 };
					let max_x = if max[X] < corner[X] + size { 0 } else { 1 };
					let max_y = if max[Y] < corner[Y] + size { 0 } else { 1 };

					let mut res = HashMap::new();
					for x in min_x..=max_x {
						for y in min_y..=max_y {
							if let Some((value, weight)) =
								children[x + y * 2].get_area(min, max, corner + Vector::new([x, y]) * size, size)
							{
								if let Some(w) = res.get_mut(&value) {
									*w += weight;
								} else {
									res.insert(value, weight);
								}
							}
						}
					}
					let mut best = None;
					let mut weight = 0;
					for (value, w) in res {
						if w > weight {
							weight = w;
							best = Some((value, w));
						}
					}
					best
				},
				Node::Leaf(value) => value.map(|v| (v, size * size)),
			}
		}
	}
}

// maybe
// - store count in initial data
//    - ground as nth point
// - options
//   - nearest query
//   - weight
//   - ...
// - some way to decay weigth for lower layers
// - test bottom-up segmentation
