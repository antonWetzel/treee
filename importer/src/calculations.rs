use math::{Dimension, Mat, Vector, X, Y, Z};
use rayon::prelude::*;

use crate::point::Point;

pub const MAX_NEIGHBORS: usize = 64 - 1;

fn edge_adjust_factor(direction: f32) -> f32 {
	// approximate in the range [0, 1] the inverse function of sin(pi*x^2) / (pi*x^2)
	const LINEAR_WEIGHT: f32 = 0.44876004;
	const POW_8_WEIGHT: f32 = 0.23475774;
	1.0 - LINEAR_WEIGHT * direction - POW_8_WEIGHT * direction.powi(8)
}

fn size(neighbors: &[(f32, usize)], points: &[Vector<3, f32>]) -> f32 {
	let position = points[neighbors[0].1];
	let (mean, direction_value) = {
		let mut mean = 0.0;
		let mut direction = Vector::<3, f32>::new([0.0, 0.0, 0.0]);
		for (_, neighbor) in neighbors[1..].iter().copied() {
			let neighbor = points[neighbor];
			let diff = position - neighbor;
			let length = diff.length();
			mean += length;
			if length < 0.01 {
				continue;
			}
			direction += diff / length;
		}
		(
			mean / (neighbors.len() - 1) as f32,
			direction.length() / (neighbors.len() - 1) as f32,
		)
	};
	0.5 * mean * edge_adjust_factor(direction_value)
}

pub fn calculate(data: Vec<Vector<3, f32>>) -> Vec<Point> {
	let neighbors = Neighbors::new(&data);

	let (min, max) = {
		let mut min = data[0][Y];
		let mut max = data[0][Y];
		for p in data.iter().skip(1) {
			if p[Y] < min {
				min = p[Y];
			} else if p[Y] > max {
				max = p[Y];
			}
		}
		(min, max)
	};

	let (slices, slice_width) = {
		let slice_width = 0.05;

		let slices = ((max - min) / slice_width).ceil() as usize;
		let mut means = vec![(Vector::new([0.0, 0.0]), 0); slices];
		for pos in data.iter().copied() {
			let idx = ((pos[Y] - min) / slice_width) as usize;
			means[idx].0 += [pos[X], pos[Z]].into();
			means[idx].1 += 1;
		}
		for mean in means.iter_mut() {
			mean.0 /= mean.1 as f32;
		}
		let mut variance = vec![0.0f32; slices];
		for pos in data.iter().copied() {
			let idx = ((pos[Y] - min) / slice_width) as usize;
			variance[idx] += (means[idx].0 - [pos[X], pos[Z]].into()).length_squared();
		}
		let mut max_var = 0.0;
		for i in 0..variance.len() {
			variance[i] /= means[i].1 as f32;
			if variance[i] > max_var {
				max_var = variance[i];
			}
		}
		let mut mapped = vec![0; slices];
		for i in 0..variance.len() {
			mapped[i] = map_to_u32(variance[i] / max_var)
		}
		(mapped, slice_width)
	};

	let sub_step = u32::MAX / data.len() as u32;

	let mut res = (0..data.len())
		.into_par_iter()
		.map(|i| {
			let neighbors = neighbors.get(i);

			let mean = {
				let mut mean = Vector::<3, f32>::new([0.0, 0.0, 0.0]);
				for (_, neighbor) in neighbors {
					mean += data[*neighbor];
				}
				mean / neighbors.len() as f32
			};
			let variance = {
				let mut variance = Mat::<3, f32>::default();
				for (_, neigbhor) in neighbors {
					let difference = data[*neigbhor] - mean;
					for x in X.to(Z) {
						for y in X.to(Z) {
							variance[x + y] += difference[x] * difference[y];
						}
					}
				}
				for x in X.to(Z) {
					for y in X.to(Z) {
						variance[x + y] /= neighbors.len() as f32;
					}
				}
				variance
			};

			let eigen_values = variance.fast_eigenvalues();
			let eigen_vectors = variance.calculate_eigenvectors(eigen_values);

			Point {
				render: render::Point {
					position: data[i],
					normal: eigen_vectors[Z],
					size: size(neighbors, &data),
				},
				slice: slices[((data[i][Y] - min) / slice_width) as usize],
				sub_index: i as u32 * sub_step,
				curve: map_to_u32((3.0 * eigen_values[Z]) / (eigen_values[X] + eigen_values[Y] + eigen_values[Z])),
			}
		})
		.collect::<Vec<Point>>();

	for _ in 0..3 {
		for i in 0..data.len() {
			let neighbors = neighbors.get(i);
			res[i].curve = neighbors
				.iter()
				.map(|n| res[n.1].curve / neighbors.len() as u32)
				.sum()
		}
	}

	res
}

struct Adapter;
impl k_nearest::Adapter<3, f32, Vector<3, f32>> for Adapter {
	fn get(point: &Vector<3, f32>, dimension: Dimension) -> f32 {
		point[dimension]
	}
	fn get_all(point: &Vector<3, f32>) -> [f32; 3] {
		point.data()
	}
}

//todo: check if precalculated is better
pub struct Neighbors(Vec<(usize, [(f32, usize); MAX_NEIGHBORS])>);

impl Neighbors {
	pub fn new(points: &[Vector<3, f32>]) -> Self {
		let kd_tree =
			k_nearest::KDTree::<3, f32, Vector<3, f32>, Adapter, k_nearest::EuclideanDistanceSquared>::new(points);

		let mut neighbors = Vec::<(usize, [(f32, usize); MAX_NEIGHBORS])>::new();
		neighbors.reserve_exact(points.len());
		unsafe { neighbors.set_len(points.len()) };
		neighbors
			.par_iter_mut()
			.zip(points)
			.for_each(|(neighbor, point)| {
				neighbor.0 = kd_tree.k_nearest(point, &mut neighbor.1, 1.0);
			});
		Self(neighbors)
	}

	pub fn get(&self, index: usize) -> &[(f32, usize)] {
		&self.0[index].1[0..self.0[index].0]
	}
}

pub fn map_to_u32(value: f32) -> u32 {
	(value * u32::MAX as f32) as u32
}
