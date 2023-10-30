use math::{Mat, Vector, X, Y, Z};

use crate::{
	tree::{Neighbors, MAX_NEIGHBORS},
	writer::Writer,
	Environment,
};

fn edge_adjust_factor(direction: f32) -> f32 {
	// approximate in the range [0, 1] the inverse function of sin(pi*x^2) / (pi*x^2)
	const LINEAR_WEIGHT: f32 = 0.44876004;
	const POW_8_WEIGHT: f32 = 0.23475774;
	1.0 - LINEAR_WEIGHT * direction - POW_8_WEIGHT * direction.powi(8)
}

fn size(neighbors: &[(f32, usize)], points: &[render::Point]) -> f32 {
	let position = points[neighbors[0].1].position;
	let (mean, direction_value) = {
		let mut mean = 0.0;
		let mut direction = Vector::<3, f32>::new([0.0, 0.0, 0.0]);
		for (_, neighbor) in neighbors[1..].iter().copied() {
			let neighbor = points[neighbor].position;
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

pub fn calculate(
	points: &mut [render::Point],
	point_properties: bool,
	neighbors: &Neighbors,
	environment: &Environment,
	node_index: usize,
	writer: &Writer,
) {
	let mut heights = Vec::with_capacity(points.len());

	let mut curve = Vec::with_capacity(points.len());

	for i in 0..points.len() {
		let neighbors = neighbors.get(i);
		let mean = {
			let mut mean = Vector::<3, f32>::new([0.0, 0.0, 0.0]);
			for (_, neighbor) in neighbors {
				mean += points[*neighbor].position;
			}
			mean / neighbors.len() as f32
		};
		let variance = {
			let mut variance = Mat::<3, f32>::default();
			for (_, neigbhor) in neighbors {
				let difference = points[*neigbhor].position - mean;
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

		if point_properties {
			points[i].normal = eigen_vectors[Z];
			points[i].size = size(neighbors, points);
		}

		{
			let height = points[i].position[Y];
			let value = (height - environment.min) / environment.diff;
			heights.push(map_to_u32(value))
		}

		{
			let value = (3.0 * eigen_values[Z]) / (eigen_values[X] + eigen_values[Y] + eigen_values[Z]);
			if neighbors.len() < MAX_NEIGHBORS {
				curve.push(u32::MAX);
			} else {
				curve.push(map_to_u32(value));
			}
		}
	}

	writer.save_property(node_index, "height", &heights);
	writer.save_property(node_index, "curve", &curve);
}

pub fn map_to_u32(value: f32) -> u32 {
	(value * u32::MAX as f32) as u32
}
