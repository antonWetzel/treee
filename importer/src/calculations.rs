use std::num::NonZeroU32;

use nalgebra as na;

use crate::{point::Point, Settings};

pub struct SegmentInformation {
	pub trunk_height: project::Value,
	pub crown_height: project::Value,
}

pub fn calculate(
	data: Vec<na::Point3<f32>>,
	segment: NonZeroU32,
	settings: &Settings,
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

	let (slices, slice_width, trunk_crown_sep) = {
		let slice_width = 0.05;

		let slices = ((height / slice_width).ceil() as usize) + 1;
		let mut means = vec![(na::vector![0.0, 0.0], 0); slices];
		for pos in data.iter().copied() {
			let idx = ((pos.y - min) / slice_width) as usize;
			means[idx].0 += na::vector![pos.x, pos.z];
			means[idx].1 += 1;
		}
		for mean in means.iter_mut() {
			mean.0 /= mean.1 as f32;
		}
		let mut variance = vec![0.0f32; slices];
		for pos in data.iter().copied() {
			let idx = ((pos.y - min) / slice_width) as usize;
			variance[idx] += (means[idx].0 - na::vector![pos.x, pos.z]).norm_squared();
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
			let percent = variance[i] / max_var;
			mapped[i] = map_to_u32(percent);
		}

		let min_slice = mapped.len() / 5;
		let sep = mapped
			.iter()
			.enumerate()
			.skip(min_slice)
			.find(|&(_, &v)| v > u32::MAX / 3)
			.map(|(index, _)| index)
			.unwrap_or(0);

		(mapped, slice_width, min + slice_width * sep as f32)
	};
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
				.sum::<f32>();
			let size = size / (neighbors.len() - 1) as f32 / 2.0;

			Point {
				render: project::Point {
					position: data[i],
					normal: eigen_vectors,
					size,
				},
				segment,
				slice: slices[((data[i].y - min) / slice_width) as usize],
				height: ((data[i].y - min) / (max - min) * u32::MAX as f32) as u32,
				curve: map_to_u32((3.0 * eigen_values.z) / (eigen_values.y + eigen_values.y + eigen_values.z)),
			}
		})
		.collect::<Vec<Point>>();

	let trunk_heigth = trunk_crown_sep - min;
	let crown_heigth = max - trunk_crown_sep;
	(
		res,
		SegmentInformation {
			trunk_height: project::Value::RelativeHeight {
				absolute: trunk_heigth,
				percent: trunk_heigth / height,
			},
			crown_height: project::Value::RelativeHeight {
				absolute: crown_heigth,
				percent: crown_heigth / height,
			},
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

#[test]
fn test() {
	let matrix = na::matrix![
		3.0, 2.0, 1.0;
		2.0, 1.0, 4.0;
		1.0, 4.0, 2.0;
	];
	{
		let start = std::time::Instant::now();
		for _ in 0..1_000_000 {
			let values = fast_eigenvalues(matrix);
			let last = calculate_last_eigenvector(matrix, values);
			std::hint::black_box((values, last));
		}
		println!("Custom: {}", start.elapsed().as_secs_f64());
	}
	{
		let start = std::time::Instant::now();
		for _ in 0..1_000_000 {
			let x = na::SymmetricEigen::new(matrix);
			std::hint::black_box((x.eigenvalues, x.eigenvectors));
		}
		println!("Nalg: {}", start.elapsed().as_secs_f64());
	}
	// panic!()
}
