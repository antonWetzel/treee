use std::{
	collections::{HashMap, VecDeque},
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
	// assert_ne!(index, nearest);

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
		*edges.entry(*edge).or_default() += 1;
	}

	let mut active_edges = VecDeque::new();
	for edge in new_edges.into_iter() {
		active_edges.push_back(edge);
	}
	while let Some(edge) = active_edges.pop_front() {
		expand(
			data,
			neighors_tree,
			edges,
			indices,
			used,
			edge,
			&mut active_edges,
		)
	}
}

fn expand(
	data: &[Vector<3, f32>],
	neighors_tree: &NeighborsTree,
	edges: &mut HashMap<Edge, usize>,
	indices: &mut Vec<u32>,
	used: &mut [bool],
	edge: Edge,
	active_edges: &mut VecDeque<Edge>,
) {
	if edges[&edge] >= 2 {
		return;
	}

	let point_a = data[edge.active_1];
	let point_b = data[edge.active_2];
	let inside = data[edge.inside];

	let alpha = inside - point_a;
	let beta = point_b - point_a;
	let up = alpha.cross(beta);
	let out = up.cross(beta).normalized();

	let neighbors = neighors_tree.get(edge.active_1);

	let (Some(third), _, _) = neighbors
		.iter()
		.skip(1)
		.copied()
		.map(|(_, idx)| idx)
		.filter(|&idx| idx != edge.active_2)
		.fold(
			(None, MAX_PROJECTION_OVERLAP, f32::MAX),
			|(third, best, best_diagonal), idx| {
				let point_c = data[idx];
				let ca = (point_a - point_c).normalized();
				let cb = (point_b - point_c).normalized();

				let c_up = ca.cross(cb);
				let c_in = beta.cross(c_up);

				if out.dot(c_in) >= 0.0 {
					return (third, best, best_diagonal);
				}
				let projection = ca.dot(cb);
				if projection > best {
					return (third, best, best_diagonal);
				}

				let diagonal = if projection == best {
					let diagonal = (point_a - point_c)
						.length()
						.max((point_b - point_c).length());
					if diagonal >= best_diagonal {
						return (third, best, best_diagonal);
					}
					diagonal
				} else {
					f32::MAX
				};

				// todo: check third neighbor of second
				// todo: check first and second neighbor of third

				(Some(idx), projection, diagonal)
			},
		)
	else {
		return;
	};

	for idx in [edge.active_1, edge.active_2, third] {
		indices.push(idx as u32);
	}
	used[third] = true;

	let new_edges = [
		Edge::new(third, edge.active_2, edge.active_1),
		Edge::new(edge.active_1, third, edge.active_2),
	];
	*edges.entry(edge).or_default() += 1;

	for edge in new_edges {
		*edges.entry(edge).or_default() += 1;
		active_edges.push_back(edge);
	}
}
