use std::{
	collections::{hash_map::Entry, HashMap, VecDeque},
	ops::Not,
};

use math::Vector;

use crate::calculations::NeighborsTree;

const MAX_PROJECTION_OVERLAP: f32 = 2.0;

pub fn triangulate(data: &[Vector<3, f32>], neighors_tree: &NeighborsTree) -> Vec<u32> {
	let mut edges = HashMap::new();
	let mut used = vec![false; data.len()];

	let mut indices = Vec::new();
	for index in 0..data.len() {
		if used[index] {
			continue;
		}
		start_triangulation(
			data,
			neighors_tree,
			index,
			&mut edges,
			&mut indices,
			&mut used,
		);
	}
	indices
}

#[derive(Clone, Copy)]
struct Edge {
	active_1: usize,
	active_2: usize,

	inside: usize,
}

impl Edge {
	fn new(active_1: usize, active_2: usize, inside: usize) -> Self {
		if active_1 < active_2 {
			Self { active_1, active_2, inside }
		} else {
			Self {
				active_1: active_2,
				active_2: active_1,
				inside,
			}
		}
	}
}

impl std::cmp::Eq for Edge {}

impl std::cmp::PartialEq for Edge {
	fn eq(&self, other: &Self) -> bool {
		self.active_1 == other.active_1 && self.active_2 == other.active_2
	}
}

impl std::hash::Hash for Edge {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.active_1.hash(state);
		self.active_2.hash(state);
	}
}

fn start_triangulation(
	data: &[Vector<3, f32>],
	neighors_tree: &NeighborsTree,
	index: usize,
	edges: &mut HashMap<Edge, usize>,
	indices: &mut Vec<u32>,
	used: &mut [bool],
) {
	let neighbors = neighors_tree.get(index);

	let mut iter = neighbors
		.iter()
		.skip(1)
		.copied()
		.map(|(_, idx)| idx)
		.filter(|idx| used[*idx].not());
	let Some(nearest) = iter.next() else {
		return;
	};
	assert_ne!(index, nearest);

	let point_a = data[index];
	let point_b = data[nearest];

	let (_, Some(third)) = iter.fold((MAX_PROJECTION_OVERLAP, None), |(best, third), index| {
		let point_c = data[index];
		let ca = (point_a - point_c).normalized();
		let cb = (point_b - point_c).normalized();
		let projection = ca.dot(cb);
		if projection < best {
			(projection, Some(index))
		} else {
			(best, third)
		}
	}) else {
		return;
	};

	let new_edges = [
		Edge::new(nearest, index, third),
		Edge::new(third, nearest, index),
		Edge::new(index, third, nearest),
	];

	for idx in [index, nearest, third] {
		used[idx] = true;
		indices.push(idx as u32);
	}

	for edge in new_edges.iter() {
		match edges.entry(*edge) {
			Entry::Vacant(v) => drop(v.insert(1)),
			Entry::Occupied(mut v) => *v.get_mut() += 1,
		}
	}

	let mut active_edges = VecDeque::new();
	for edge in new_edges.into_iter() {
		active_edges.push_back(edge);
	}
	while let Some(_edge) = active_edges.pop_front() {
		//expand
	}
}
