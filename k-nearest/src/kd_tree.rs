use std::cmp::Ordering;

use crate::{best_set::BestSet, Adapter, Metric};

pub struct KDTree<const N: usize, Value, Point, Ada, Met>
where
	Value: Copy + Default + PartialOrd,
	Ada: Adapter<N, Value, Point>,
	Met: Metric<N, Value>,
{
	tree: Vec<Option<([Value; N], usize)>>,
	phantom: std::marker::PhantomData<(Point, Ada, Met)>,
}

impl<const N: usize, Value, Point, Ada, Met> KDTree<N, Value, Point, Ada, Met>
where
	Value: Copy + Default + PartialOrd,
	Ada: Adapter<N, Value, Point>,
	Met: Metric<N, Value>,
{
	pub fn new(data: &[Point]) -> Self {
		let next_power_2 = 1usize << (usize::BITS - (data.len() - 1).leading_zeros());
		let mut tree = vec![None; next_power_2];
		let mut positions = Vec::with_capacity(data.len());
		for (i, p) in data.iter().enumerate() {
			positions.push((Ada::get_all(p), i));
		}
		Self::create_tree(0, 0, &mut tree, 0, positions.len() - 1, &mut positions);
		KDTree { tree, phantom: std::marker::PhantomData }
	}

	fn create_tree(
		index: usize,
		dim: usize,
		tree: &mut [Option<([Value; N], usize)>],
		lower: usize,
		upper: usize,
		positions: &mut [([Value; N], usize)],
	) {
		// choose middle with bias to the right, so recursion is deeper on the left
		//   bias direction must match median finding scheme
		let middle = (upper - lower + 1) / 2 + lower;
		Self::partition(middle, dim, lower, upper, positions);
		tree[index] = Some((positions[middle].0, positions[middle].1));
		let next_dim = (dim + 1) % N;
		if lower < middle {
			Self::create_tree(index * 2 + 1, next_dim, tree, lower, middle - 1, positions);
		}
		if middle < upper {
			Self::create_tree(index * 2 + 2, next_dim, tree, middle + 1, upper, positions);
		}
	}

	fn partition(target: usize, dim: usize, mut lower: usize, mut upper: usize, positions: &mut [([Value; N], usize)]) {
		loop {
			// maybe: use 2 pointer variant
			let pivot_index = (upper - lower) / 2 + lower;
			let pivot = positions[pivot_index].0[dim];
			positions.swap(pivot_index, upper);
			let mut store = lower;
			for i in lower..upper {
				if positions[i].0[dim] < pivot {
					positions.swap(store, i);
					store += 1;
				}
			}
			positions.swap(store, upper);
			match target.cmp(&store) {
				Ordering::Equal => break,
				Ordering::Less => upper = store - 1,
				Ordering::Greater => lower = store + 1,
			}
		}
	}

	pub fn k_nearest(&self, point: &Point, data: &mut [(Value, usize)], max_distance: Value) -> usize {
		self.nearest_to_position(&Ada::get_all(point), data, max_distance)
	}

	pub fn nearest_to_position(
		&self,
		position: &[Value; N],
		data: &mut [(Value, usize)],
		max_distance: Value,
	) -> usize {
		let mut best_set = BestSet::new(max_distance, data);
		self.search_nearest(0, position, 0, &mut best_set);
		best_set.result()
	}

	fn search_nearest(&self, index: usize, position: &[Value; N], dim: usize, best_set: &mut BestSet<Value>) {
		if index >= self.tree.len() {
			return;
		}
		let (point, point_index) = match self.tree[index] {
			Some(v) => v,
			None => return,
		};
		let diff = Met::distance(&point, position);
		if diff < best_set.distance() {
			best_set.insert((diff, point_index));
		}
		let next_dim = (dim + 1) % N;
		let is_left = position[dim] < point[dim];
		if is_left {
			self.search_nearest(index * 2 + 1, position, next_dim, best_set);
		} else {
			self.search_nearest(index * 2 + 2, position, next_dim, best_set);
		}
		if Met::distance_plane(position, point[dim], dim) < best_set.distance() {
			if is_left {
				self.search_nearest(index * 2 + 2, position, next_dim, best_set);
			} else {
				self.search_nearest(index * 2 + 1, position, next_dim, best_set);
			}
		}
	}
}
