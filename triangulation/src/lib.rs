use std::{
	collections::{HashSet, VecDeque},
	sync::mpsc::SendError,
};

use nalgebra as na;

pub struct Adapter;

impl k_nearest::Adapter<3, f32, na::Point<f32, 3>> for Adapter {
	fn get(point: &na::Point<f32, 3>, dimension: usize) -> f32 {
		point[dimension]
	}

	fn get_all(point: &na::Point<f32, 3>) -> [f32; 3] {
		point.coords.data.0[0]
	}
}

type Tree = k_nearest::KDTree<3, f32, na::Point<f32, 3>, Adapter, k_nearest::EuclideanDistanceSquared>;

type Package = Option<na::Point<usize, 3>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
	Blocked,
	Used,
	Free,
}

pub fn triangulate(
	data: &[na::Point<f32, 3>],
	alpha: f32,
	sub_sample_distance: f32,
	res: std::sync::mpsc::Sender<Package>,
) -> Result<(), SendError<Package>> {
	let tree = Tree::new(data);
	let mut used = vec![State::Free; data.len()];
	for index in 0..used.len() {
		if used[index] == State::Blocked {
			continue;
		}
		let around = tree.nearest(&data[index], sub_sample_distance.powi(2));
		for entry in around.into_iter().skip(1) {
			used[entry.index] = State::Blocked
		}
	}

	let mut found = HashSet::new();
	while let Some(seed) = seed(data, &used, &tree, alpha, &res) {
		res.send(Some(seed))?;
		used[seed.x] = State::Used;
		used[seed.y] = State::Used;
		used[seed.z] = State::Used;

		let mut edges = [
			(Edge::new(seed.x, seed.z), seed.y),
			(Edge::new(seed.z, seed.y), seed.x),
			(Edge::new(seed.y, seed.x), seed.z),
		]
		.into_iter()
		.collect::<VecDeque<_>>();

		while let Some(edge) = edges.pop_front() {
			if found.contains(&edge) {
				continue;
			}
			found.insert(edge);
			let (first, second) = (edge.0.active_1, edge.0.active_2);
			let old = edge.1;
			if let Some(third) = find_third(data, first, second, &tree, old, alpha, &used) {
				// checked in `find_third`
				if third == old {
					continue;
				}
				res.send(Some([first, second, third].into()))?;
				used[third] = State::Used;
				for edge in [
					(Edge::new(first, third), second),
					(Edge::new(third, second), first),
				] {
					if found.contains(&edge) {
						continue;
					}
					edges.push_back(edge);
				}
			}
		}
	}
	Ok(())
}

#[derive(Clone, Copy)]
struct Edge {
	active_1: usize,
	active_2: usize,
}

impl Edge {
	fn new(active_1: usize, active_2: usize) -> Self {
		Self { active_1, active_2 }
	}
}

impl std::cmp::Eq for Edge {}

impl std::cmp::PartialEq for Edge {
	fn eq(&self, other: &Self) -> bool {
		self.active_1 == other.active_1 && self.active_2 == other.active_2
			|| self.active_1 == other.active_2 && self.active_2 == other.active_1
	}
}

impl std::hash::Hash for Edge {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		if self.active_1 < self.active_2 {
			self.active_1.hash(state);
			self.active_2.hash(state);
		} else {
			self.active_2.hash(state);
			self.active_1.hash(state);
		}
	}
}

fn seed(
	data: &[na::Point<f32, 3>],
	used: &[State],
	tree: &Tree,
	alpha: f32,
	res: &std::sync::mpsc::Sender<Package>,
) -> Option<na::Point<usize, 3>> {
	for (first, point) in data
		.iter()
		.enumerate()
		.filter(|&(idx, _)| used[idx] == State::Free)
	{
		match res.send(None) {
			Ok(_) => {},
			Err(_) => return None,
		}
		let nearest = tree.nearest(point, (2.0 * alpha).powi(2));
		if nearest.len() <= 2 {
			continue;
		}
		for (second_index, second) in nearest
			.iter()
			.enumerate()
			.skip(1)
			.filter(|(_, entry)| used[entry.index] == State::Free)
		{
			for third in nearest
				.iter()
				.skip(second_index + 1)
				.filter(|entry| used[entry.index] == State::Free)
			{
				let Some(center) = sphere_location(data[first], data[second.index], data[third.index], alpha) else {
					continue;
				};

				if tree.empty(&center, (alpha - 0.001).powi(2)) {
					return Some([first, second.index, third.index].into());
				}

				let Some(center) = sphere_location(data[second.index], data[first], data[third.index], alpha) else {
					continue;
				};

				if tree.empty(&center, (alpha - 0.001).powi(2)) {
					return Some([second.index, first, third.index].into());
				}
			}
		}
	}
	None
}

fn find_third(
	data: &[na::Point<f32, 3>],
	first: usize,
	second: usize,
	tree: &Tree,
	old: usize,
	alpha: f32,
	used: &[State],
) -> Option<usize> {
	let a = data[first];
	let b = data[old];
	let c = data[second];
	let center = sphere_location(a, b, c, alpha).unwrap();
	let bar = (c - a).normalize();
	let mid_point = na::center(&a, &c);
	let to_center = (center - mid_point).normalize();

	let search_distance = alpha + (alpha.powi(2) - (a.coords - mid_point.coords).norm_squared()).sqrt();

	let nearest = tree.nearest(&mid_point, search_distance.powi(2));
	let mut best = None;
	let mut best_angle = std::f32::consts::TAU;
	for third in nearest
		.iter()
		.skip(1)
		.filter(|entry| used[entry.index] != State::Blocked)
		.filter(|entry| entry.index != first && entry.index != second && entry.index != old)
	{
		let Some(center_2) = sphere_location(data[first], data[second], data[third.index], alpha) else {
			continue;
		};
		let to_center_2 = (center_2 - mid_point).normalize();
		let angle = to_center.dot(&to_center_2).clamp(-1.0, 1.0).acos();
		let angle = if to_center.cross(&to_center_2).dot(&bar) < 0.0 {
			std::f32::consts::TAU - angle
		} else {
			angle
		};
		if angle >= best_angle {
			continue;
		}

		best_angle = angle;
		best = Some(third.index);
	}
	best
}

/// https://stackoverflow.com/a/34326390
fn sphere_location(
	point_a: na::Point<f32, 3>,
	point_b: na::Point<f32, 3>,
	point_c: na::Point<f32, 3>,
	alpha: f32,
) -> Option<na::Point<f32, 3>> {
	let ac = point_c - point_a;
	let ab = point_b - point_a;
	let out = ab.cross(&ac);

	let to = (out.cross(&ab) * ac.norm_squared() + ac.cross(&out) * ab.norm_squared()) / (2.0 * out.norm_squared());
	let circumcenter = point_a + to;

	let dist = alpha * alpha - to.norm_squared();
	if dist <= 0.0 {
		return None;
	}
	Some(circumcenter - out.normalize() * dist.sqrt())
}
