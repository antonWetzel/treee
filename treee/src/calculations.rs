use nalgebra as na;

use crate::segmenting::Tree;

#[derive(Debug, Clone, Copy)]
pub struct SegmentInformation {
	pub ground_sep: f32,
	pub crown_sep: f32,
	pub trunk_diameter: f32,
	pub crown_diameter: f32,
}

impl SegmentInformation {
	pub fn new(data: &[na::Point3<f32>], min: f32, max: f32) -> Self {
		let height = max - min;

		// let slice_width = settings.calculations_slice_width;
		let slice_width = 0.1;
		let ground_max_search_height = 1.0;
		let ground_min_area_scale = 1.5;
		let trunk_diameter_height = 1.3;
		let trunk_diameter_range = 0.2;
		let crown_diameter_difference = 1.0;

		let slices = ((height / slice_width) as usize) + 1;
		let mut sets = vec![<Option<Tree>>::None; slices];
		for pos in data.iter().copied() {
			let idx = ((pos.y - min) / slice_width) as usize;
			match &mut sets[idx] {
				Some(tree) => tree.insert(na::vector![pos.x, pos.z].into()),
				x @ None => *x = Some(Tree::new(na::vector![pos.x, pos.z].into())),
			}
		}

		let areas = sets
			.into_iter()
			.map(|set| set.map(|set| set.statistics().area).unwrap_or(0.0))
			.collect::<Vec<_>>();
		let min_area = areas
			.iter()
			.copied()
			.skip((1.0 / slice_width) as usize)
			.take((10.0 / slice_width) as usize)
			.min_by(|a, b| a.total_cmp(b))
			.unwrap_or(0.5)
			.max(0.5);
		let ground = areas
			.iter()
			.copied()
			.enumerate()
			.take((ground_max_search_height / slice_width) as usize)
			.find(|&(_, area)| area > min_area * ground_min_area_scale)
			.map(|(idx, _)| idx);
		let ground_sep = if let Some(ground) = ground {
			areas
				.iter()
				.enumerate()
				.take(slices / 2)
				.skip(ground)
				.find(|&(_, &v)| v < min_area * ground_min_area_scale)
				.map(|(index, _)| index)
				.unwrap_or(0)
		} else {
			0
		};

		let (trunk_diameter, trunk_max) = {
			let trunk_min = ground_sep as f32 * slice_width + trunk_diameter_height - 0.5 * trunk_diameter_range;
			let trunk_max = trunk_min + trunk_diameter_range;
			let slice_trunk = data
				.iter()
				.filter(|p| (trunk_min..trunk_max).contains(&(p.y - min)))
				.map(|p| na::Point2::new(p.x, p.y))
				.collect::<Vec<_>>();

			let mut best_score = f32::MAX;
			let mut best_circle = (0.5, na::Point2::new(0.0, 0.0));
			if slice_trunk.len() >= 8 {
				for _ in 0..1000 {
					let x = slice_trunk[rand::random::<usize>() % slice_trunk.len()];
					let y = slice_trunk[rand::random::<usize>() % slice_trunk.len()];
					let z = slice_trunk[rand::random::<usize>() % slice_trunk.len()];
					let Some((center, radius)) = circle(x, y, z) else {
						continue;
					};
					let score = slice_trunk
						.iter()
						.map(|p| ((p - center).norm() - radius).abs().min(0.2))
						.sum::<f32>();
					if score < best_score {
						best_score = score;
						best_circle = (2.0 * radius, center);
					}
				}
			}

			(best_circle.0, (trunk_max / slice_width).ceil() as usize)
		};

		let min_crown_area = std::f32::consts::PI * ((trunk_diameter + crown_diameter_difference) / 2.0).powi(2);

		let crown_sep = areas
			.iter()
			.enumerate()
			.skip(trunk_max)
			.find(|&(_, &v)| v > min_crown_area)
			.map(|(index, _)| index)
			.unwrap_or(0);

		let crown_area = areas
			.iter()
			.copied()
			.skip(crown_sep)
			.max_by(|a, b| a.total_cmp(b))
			.unwrap_or(0.0);

		Self {
			ground_sep: min + ground_sep as f32 * slice_width,
			crown_sep: min + crown_sep as f32 * slice_width,
			trunk_diameter,
			crown_diameter: approximate_diameter(crown_area),
		}
	}
}

/// https://stackoverflow.com/a/34326390
/// adopted for 2D
fn circle(
	point_a: na::Point2<f32>,
	point_b: na::Point2<f32>,
	point_c: na::Point2<f32>,
) -> Option<(na::Point2<f32>, f32)> {
	let ac = point_c - point_a;
	let ab = point_b - point_a;
	let bc = point_c - point_b;
	if ab.dot(&ac) < 0.0 || ac.dot(&bc) < 0.0 || ab.dot(&bc) > 0.0 {
		return None;
	}

	let cross = ab.x * ac.y - ab.y * ac.x;
	let to =
		(na::vector![-ab.y, ab.x] * ac.norm_squared() + na::vector![ac.y, -ac.x] * ab.norm_squared()) / (2.0 * cross);
	let radius = to.norm();
	if radius.is_nan() {
		return None;
	}
	Some((point_a + to, radius))
}

fn approximate_diameter(area: f32) -> f32 {
	2.0 * (area / std::f32::consts::PI).sqrt()
}
