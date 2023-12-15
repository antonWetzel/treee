use std::collections::HashSet;

use math::Vector;

use crate::calculations::NeighborsTree;

const MAX_CONNECTIONS: usize = 12;
const ALMOST_MAX_CONECTIONS: usize = 10;

// const NEIGHBOR_LIMIT: usize = 12;
const MAX_DISTANCE: f32 = 0.1;
const MAX_PROJECTION_OVERLAP: f32 = 2.0;

pub fn triangulate(data: &[Vector<3, f32>], neighors_tree: &NeighborsTree) -> Vec<u32> {
	let mut used = vec![0; data.len()];

	let mut indices = Vec::new();
	for index in 0..data.len() {
		if used[index] != 0 {
			continue;
		}
		start_triangulation(data, neighors_tree, index, &mut indices, &mut used);
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
	indices: &mut Vec<u32>,
	used: &mut [usize],
) {
	let neighbors = neighors_tree.get(index);

	let mut iter = neighbors
		.iter()
		.skip(1)
		.copied()
		.map(|entry| entry.index)
		.filter(|&idx| used[idx] == 0);
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
		used[idx] += 1;
		indices.push(idx as u32);
	}

	let mut edges = HashSet::new();

	for edge in new_edges.into_iter() {
		edges.insert(edge);
	}
	while let Some(edge) = edges.iter().copied().next() {
		edges.remove(&edge);
		expand(data, neighors_tree, &mut edges, indices, used, edge)
	}
}

fn expand(
	data: &[Vector<3, f32>],
	neighors_tree: &NeighborsTree,
	edges: &mut HashSet<Edge>,
	indices: &mut Vec<u32>,
	used: &mut [usize],
	edge: Edge,
) {
	if used[edge.active_1] >= MAX_CONNECTIONS || used[edge.active_2] >= MAX_CONNECTIONS {
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
		.filter(|entry| entry.distance < MAX_DISTANCE * MAX_DISTANCE)
		// .take(NEIGHBOR_LIMIT)
		.map(|entry| entry.index)
		.filter(|&idx| idx != edge.active_2)
		.filter(|&idx| used[idx] < MAX_CONNECTIONS)
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
		used[idx] += 1;
	}

	let edge_1 = Edge::new(third, edge.active_2, edge.active_1);
	if edges.contains(&edge_1) {
		edges.remove(&edge_1);
		used[edge.active_2] = used[edge.active_2].max(ALMOST_MAX_CONECTIONS);
	} else {
		edges.insert(edge_1);
	}

	let edge_2 = Edge::new(edge.active_1, third, edge.active_2);
	if edges.contains(&edge_2) {
		edges.remove(&edge_2);
		used[edge.active_1] = used[edge.active_1].max(ALMOST_MAX_CONECTIONS);
	} else {
		edges.insert(edge_2);
	}
}
