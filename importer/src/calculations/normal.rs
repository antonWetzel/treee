use math::{Mat, Vector, X, Z};

pub fn calculate(neighbors: &[(f32, usize)], points: &[render::Point]) -> Vector<3, f32> {
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
		// skip dividing values by neighbors.len() because only the direction is relevant
		variance
	};

	let eigen_values = variance.fast_eigenvalues();
	variance.calculate_last_eigenvector(eigen_values)
}
