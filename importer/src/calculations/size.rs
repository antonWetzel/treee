use math::Vector;

fn edge_adjust_factor(direction: f32) -> f32 {
	// approximate in the range [0, 1] the inverse function of sin(pi*x^2) / (pi*x^2)
	const LINEAR_WEIGHT: f32 = 0.44876004;
	const POW_8_WEIGHT: f32 = 0.23475774;
	1.0 - LINEAR_WEIGHT * direction - POW_8_WEIGHT * direction.powi(8)
}

pub fn calculate(neighbors: &[(f32, usize)], points: &[render::Point]) -> f32 {
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
