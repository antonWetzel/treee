use std::{num::NonZeroU32, ops::Not};

use nalgebra as na;

use crate::{point::Point, segment::Tree, Settings};

pub struct SegmentInformation {
	pub total_height: project::Value,
	pub trunk_height: project::Value,
	pub crown_height: project::Value,
	pub trunk_diameter: project::Value,
	pub crown_diameter: project::Value,
	pub latitude: project::Value,
	pub longitude: project::Value,
	pub elevation: project::Value,
}

//todo: add geojson to settings
// - get good defaults
// - validate calculation

pub fn calculate(
	data: Vec<na::Point3<f32>>,
	segment: NonZeroU32,
	settings: &Settings,
	projection: &Option<(proj4rs::Proj, proj4rs::Proj)>,
	world_offset: na::Point3<f64>,
) -> (Vec<Point>, SegmentInformation) {
	let neighbors_tree = NeighborsTree::new(&data);

	let (min, max) = {
		let mut min = data[0].y;
		let mut max = data[0].y;
		for p in data.iter().skip(1) {
			if p.y < min {
				min = p.y;
			} else if p.y > max {
				max = p.y;
			}
		}
		(min, max)
	};
	let height = max - min;

	let slice_width = settings.calculations_slice_width;

	let slices = ((height / slice_width) as usize) + 1;
	let mut sets = vec![<Option<Tree>>::None; slices];
	for pos in data.iter().copied() {
		let idx = ((pos.y - min) / slice_width) as usize;
		match &mut sets[idx] {
			Some(tree) => tree.insert(na::vector![pos.x, pos.z].into(), 0.0),
			x @ None => *x = Some(Tree::new(na::vector![pos.x, pos.z].into(), 0.0)),
		}
	}

	// let mut svg = std::fs::File::create("test.svg").unwrap();
	// svg.write_all(b"<svg viewbox=\"0 0 200 200\" xmlns=\"http://www.w3.org/2000/svg\">\n")
	// 	.unwrap();
	// for (index, set) in sets.iter().enumerate() {
	// 	if let Some(set) = set {
	// 		let a = index as f32 / sets.len() as f32;
	// 		let color = format!("rgb({}, {}, {})", a * 256.0, 0.0, (1.0 - a) * 256.0);
	// 		svg.write_all(b"  <polygon points=\"").unwrap();
	// 		let y = (sets.len() - index) as f32 * slice_width * 10.0;
	// 		for &p in set.points() {
	// 			svg.write_all(format!("{},{} ", 80.0 + p.x * 10.0, y + p.y * 5.0).as_bytes())
	// 				.unwrap();
	// 		}
	// 		svg.write_all(
	// 			format!(
	// 				"\" fill=\"{}\" stroke-width=\"0.2\" stroke=\"black\" />\n",
	// 				color
	// 			)
	// 			.as_bytes(),
	// 		)
	// 		.unwrap();

	// 		let area = set.statistics().area;

	// 		svg.write_all(
	// 			format!(
	// 				"  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke-width=\"0.1\" stroke=\"black\" />\n",
	// 				200.0,
	// 				y,
	// 				area * 2.0,
	// 				slice_width * 10.0,
	// 				color,
	// 			)
	// 			.as_bytes(),
	// 		)
	// 		.unwrap();
	// 	}
	// }
	// let y = (sets.len() + 1) as f32 * slice_width * 10.0;
	// svg.write_all(
	// 	format!(
	// 		"  <line x1=\"200\" y1=\"0\" x2=\"200\" y2=\"{}\" stroke=\"black\" />\n",
	// 		y
	// 	)
	// 	.as_bytes(),
	// )
	// .unwrap();
	// svg.write_all(b"</svg>\n").unwrap();

	let areas = sets
		.into_iter()
		.map(|set| set.map(|set| set.statistics().area).unwrap_or(0.0))
		.collect::<Vec<_>>();
	let max_area = areas
		.iter()
		.copied()
		.max_by(|a, b| a.total_cmp(b))
		.unwrap_or(1.0);
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
		.take((settings.ground_max_search_height / slice_width) as usize)
		.find(|&(_, area)| area > min_area * settings.ground_min_area_scale)
		.map(|(idx, _)| idx);
	let ground_sep = if let Some(ground) = ground {
		areas
			.iter()
			.enumerate()
			.take(slices / 2)
			.skip(ground)
			.find(|&(_, &v)| v < min_area * settings.ground_min_area_scale)
			.map(|(index, _)| index)
			.unwrap_or(0)
	} else {
		0
	};

	let (trunk_diameter, trunk_center, trunk_min, trunk_max) = {
		let trunk_min =
			ground_sep as f32 * slice_width + settings.trunk_diameter_height - 0.5 * settings.trunk_diameter_range;
		let trunk_max = trunk_min + settings.trunk_diameter_range;
		let slice_trunk = data
			.iter()
			.filter(|p| (trunk_min..trunk_max).contains(&(p.y - min)))
			.map(|p| na::Point2::new(p.x, p.y))
			.collect::<Vec<_>>();

		let mut best_score = f32::MAX;
		let mut best_circle = (0.5, na::Point2::new(0.0, 0.0));
		if slice_trunk.is_empty().not() {
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

		(
			best_circle.0,
			best_circle.1,
			(trunk_min / slice_width) as usize,
			(trunk_max / slice_width).ceil() as usize,
		)
	};

	let min_crown_area = std::f32::consts::PI * ((trunk_diameter + settings.crown_diameter_difference) / 2.0).powi(2);

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

	let mapped = areas
		.into_iter()
		.map(|area| map_to_u32(area / max_area))
		.collect::<Vec<_>>();

	let slices = mapped;

	let mut neighbors_location = bytemuck::zeroed_vec(settings.neighbors_count);

	let res = (0..data.len())
		.map(|i| {
			let neighbors = neighbors_tree.get(
				i,
				&data,
				&mut neighbors_location,
				settings.neighbors_max_distance,
			);

			let mean = {
				let mut mean = na::Point3::new(0.0, 0.0, 0.0);
				for entry in neighbors {
					mean += data[entry.index].coords;
				}
				mean / neighbors.len() as f32
			};
			let variance = {
				let mut variance = na::Matrix3::default();
				for entry in neighbors {
					let difference = data[entry.index] - mean;
					for x in 0..3 {
						for y in 0..3 {
							variance[(x, y)] += difference[x] * difference[y];
						}
					}
				}
				for x in 0..3 {
					for y in 0..3 {
						variance[(x, y)] /= neighbors.len() as f32;
					}
				}
				variance
			};

			let eigen_values = fast_eigenvalues(variance);
			let eigen_vectors = calculate_last_eigenvector(variance, eigen_values);

			let size = neighbors[1..]
				.iter()
				.copied()
				.map(|entry| entry.distance.sqrt())
				.sum::<f32>()
				.max(0.01);

			let size = size / (neighbors.len() - 1) as f32 / 2.0;
			let idx = ((data[i].y - min) / slice_width) as usize;
			let classification = if (trunk_min..trunk_max).contains(&idx) {
				1
			} else if idx <= ground_sep {
				0
			} else if idx <= crown_sep {
				2
			} else {
				3
			};

			Point {
				render: project::Point {
					position: data[i],
					normal: eigen_vectors,
					size,
				},
				segment,
				classification,
				slice: slices[idx],
				height: ((data[i].y - min) / (max - min) * u32::MAX as f32) as u32,
				curve: map_to_u32((3.0 * eigen_values.z) / (eigen_values.y + eigen_values.y + eigen_values.z)),
			}
		})
		.collect::<Vec<Point>>();

	let total_height = height - ground_sep as f32 * slice_width;
	let trunk_height = (crown_sep.saturating_sub(ground_sep)) as f32 * slice_width;
	let crown_height = total_height - trunk_height;

	let mut pos = (
		world_offset.x + trunk_center.x as f64,
		-(world_offset.z + trunk_center.y as f64),
	);
	let (latitude, longitude) = if let Some((from, to)) = projection {
		proj4rs::transform::transform(from, to, &mut pos).unwrap();
		(
			project::Value::Degrees(pos.1.to_degrees()),
			project::Value::Degrees(pos.0.to_degrees()),
		)
	} else {
		(
			project::Value::AbsolutePosition(pos.1),
			project::Value::Degrees(pos.0),
		)
	};

	(
		res,
		SegmentInformation {
			total_height: project::Value::Meters(total_height),
			trunk_height: project::Value::RelativeHeight {
				absolute: trunk_height,
				percent: trunk_height / total_height,
			},
			crown_height: project::Value::RelativeHeight {
				absolute: crown_height,
				percent: crown_height / total_height,
			},
			trunk_diameter: project::Value::Meters(trunk_diameter),
			crown_diameter: project::Value::Meters(2.0 * (crown_area / std::f32::consts::PI).sqrt()),
			latitude,
			longitude,
			elevation: project::Value::Meters(world_offset.y as f32 + min + ground_sep as f32 * slice_width),
		},
	)
}

pub struct Adapter;

impl k_nearest::Adapter<3, f32, na::Point3<f32>> for Adapter {
	fn get(point: &na::Point3<f32>, dimension: usize) -> f32 {
		point[dimension]
	}

	fn get_all(point: &na::Point3<f32>) -> [f32; 3] {
		point.coords.data.0[0]
	}
}

pub struct NeighborsTree {
	tree: k_nearest::KDTree<3, f32, na::Point3<f32>, Adapter, k_nearest::EuclideanDistanceSquared>,
}

impl NeighborsTree {
	pub fn new(points: &[na::Point3<f32>]) -> Self {
		let tree =
			<k_nearest::KDTree<3, f32, na::Point3<f32>, Adapter, k_nearest::EuclideanDistanceSquared>>::new(points);

		Self { tree }
	}

	pub fn get<'a>(
		&self,
		index: usize,
		data: &[na::Point3<f32>],
		location: &'a mut [k_nearest::Entry<f32>],
		max_distance: f32,
	) -> &'a [k_nearest::Entry<f32>] {
		let l = self.tree.k_nearest(&data[index], location, max_distance);
		&location[0..l]
	}
}

pub fn map_to_u32(value: f32) -> u32 {
	(value * u32::MAX as f32) as u32
}

// https://en.wikipedia.org/wiki/Eigenvalue_algorithm#3%C3%973_matrices
// the matrix must be real and symmetric
pub fn fast_eigenvalues(mat: na::Matrix3<f32>) -> na::Point3<f32> {
	fn square(x: f32) -> f32 {
		x * x
	}

	// I would choose better names for the variables if I know what they mean
	let p1 = square(mat[(0, 1)]) + square(mat[(0, 2)]) + square(mat[(1, 2)]);
	if p1 == 0.0 {
		return [mat[(0, 0)], mat[(1, 1)], mat[(2, 2)]].into();
	}

	let q = (mat[(0, 0)] + mat[(1, 1)] + mat[(2, 2)]) / 3.0;
	let p2 = square(mat[(0, 0)] - q) + square(mat[(1, 1)] - q) + square(mat[(2, 2)] - q) + 2.0 * p1;
	let p = (p2 / 6.0).sqrt();
	let mut mat_b = mat;
	for i in 0..3 {
		mat_b[(i, i)] -= q;
	}
	let r = mat_b.determinant() / 2.0 * p.powi(-3);
	let phi = if r <= -1.0 {
		std::f32::consts::PI / 3.0
	} else if r >= 1.0 {
		0.0
	} else {
		r.acos() / 3.0
	};

	let eig_1 = q + 2.0 * p * phi.cos();
	let eig_3 = q + 2.0 * p * (phi + (2.0 * std::f32::consts::PI / 3.0)).cos();
	let eig_2 = 3.0 * q - eig_1 - eig_3;
	[eig_1, eig_2, eig_3].into()
}

pub fn calculate_last_eigenvector(mat: na::Matrix3<f32>, eigen_values: na::Point3<f32>) -> na::Vector3<f32> {
	let mut eigen_vector = na::Vector3::<f32>::default();
	for j in 0..3 {
		for k in 0..3 {
			eigen_vector[j] += (mat[(k, j)] - if k == j { eigen_values.x } else { 0.0 })
				* (mat[(2, k)] - if 2 == k { eigen_values.y } else { 0.0 });
		}
	}
	eigen_vector.normalize()
}

// #[test]
// fn test() {
// 	let matrix = na::matrix![
// 		3.0, 2.0, 1.0;
// 		2.0, 1.0, 4.0;
// 		1.0, 4.0, 2.0;
// 	];
// 	{
// 		let start = std::time::Instant::now();
// 		for _ in 0..1_000_000 {
// 			let values = fast_eigenvalues(matrix);
// 			let last = calculate_last_eigenvector(matrix, values);
// 			std::hint::black_box((values, last));
// 		}
// 		println!("Custom: {}", start.elapsed().as_secs_f64());
// 	}
// 	{
// 		let start = std::time::Instant::now();
// 		for _ in 0..1_000_000 {
// 			let x = na::SymmetricEigen::new(matrix);
// 			std::hint::black_box((x.eigenvalues, x.eigenvectors));
// 		}
// 		println!("Nalg: {}", start.elapsed().as_secs_f64());
// 	}
// 	panic!()
// }

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
