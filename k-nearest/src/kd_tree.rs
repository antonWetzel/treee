use std::cmp::Ordering;

use crate::{best_set::BestSet, Adapter, Metric};

#[derive(Clone, Copy)]
pub struct Entry<Value> {
	pub distance: Value,
	pub index: usize,
}

unsafe impl<Value: bytemuck::Pod> bytemuck::Pod for Entry<Value> {}
unsafe impl<Value: bytemuck::Zeroable> bytemuck::Zeroable for Entry<Value> {}

pub struct KDTree<const N: usize, Value, Point, Ada, Met>
where
	Value: Copy + Default + PartialOrd,
	Ada: Adapter<N, Value, Point>,
	Met: Metric<N, Value>,
{
	tree: Vec<([Value; N], usize)>,
	phantom: std::marker::PhantomData<(Point, Ada, Met)>,
}

impl<const N: usize, Value, Point, Ada, Met> KDTree<N, Value, Point, Ada, Met>
where
	Value: Copy + Default + PartialOrd,
	[Value; N]: Default,
	Ada: Adapter<N, Value, Point>,
	Met: Metric<N, Value>,
{
	pub fn new(data: &[Point]) -> Self {
		let mut tree = Vec::with_capacity(data.len());
		for (i, p) in data.iter().enumerate() {
			tree.push((Ada::get_all(p), i));
		}
		Self::create_tree(0, &mut tree);
		KDTree { tree, phantom: std::marker::PhantomData }
	}

	fn create_tree(dim: usize, tree: &mut [([Value; N], usize)]) {
		// choose middle with bias to the right, so recursion is deeper on the left
		//   bias direction must match median finding scheme
		let middle = tree.len() / 2;
		Self::partition(middle, dim, tree);
		let next_dim = (dim + 1) % N;
		if 0 < middle {
			Self::create_tree(next_dim, &mut tree[..middle]);
		}
		if middle < tree.len() - 1 {
			Self::create_tree(next_dim, &mut tree[(middle + 1)..]);
		}
	}

	fn partition(target: usize, dim: usize, positions: &mut [([Value; N], usize)]) {
		let mut lower = 0;
		let mut upper = positions.len() - 1;
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

	pub fn k_nearest(&self, point: &Point, data: &mut [Entry<Value>], max_distance: Value) -> usize {
		self.nearest_to_position(&Ada::get_all(point), data, max_distance)
	}

	pub fn nearest_to_position(&self, position: &[Value; N], data: &mut [Entry<Value>], max_distance: Value) -> usize {
		let mut best_set = BestSet::new(max_distance, data);
		Self::search_nearest(&self.tree, position, 0, &mut best_set);
		best_set.result()
	}

	fn search_nearest(tree: &[([Value; N], usize)], position: &[Value; N], dim: usize, best_set: &mut BestSet<Value>) {
		//linear search small sections
		if tree.len() < 32 {
			for &(point, point_index) in tree {
				let diff = Met::distance(&point, position);
				if diff < best_set.distance() {
					best_set.insert(Entry { distance: diff, index: point_index });
				}
			}
			return;
		}

		//check median point
		let middle = tree.len() / 2;
		let (point, point_index) = tree[middle];
		let diff = Met::distance(&point, position);
		if diff < best_set.distance() {
			best_set.insert(Entry { distance: diff, index: point_index });
		}

		//always recurse into the section with the point
		let next_dim = (dim + 1) % N;
		let is_left = position[dim] < point[dim];
		if is_left {
			Self::search_nearest(&tree[..middle], position, next_dim, best_set);
		} else {
			Self::search_nearest(&tree[(middle + 1)..], position, next_dim, best_set);
		}

		//only recurve into the section without the point if the distance is less then to the current worst point found
		if Met::distance_plane(position, point[dim], dim) < best_set.distance() {
			if is_left {
				Self::search_nearest(&tree[(middle + 1)..], position, next_dim, best_set);
			} else {
				Self::search_nearest(&tree[..middle], position, next_dim, best_set);
			}
		}
	}
}
