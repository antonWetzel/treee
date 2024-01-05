use std::num::NonZeroU32;

use math::{ Dimension, Mat, Vector, X, Y, Z };

use crate::{ point::Point, Settings };


pub struct SegmentInformation {
	pub trunk_height: common::Value,
	pub crown_height: common::Value,
}


pub fn calculate(data: Vec<Vector<3, f32>>, segment: NonZeroU32, settings: &Settings) -> (Vec<Point>, SegmentInformation) {
	let neighbors_tree = NeighborsTree::new(&data);

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
	let height = max - min;

	let (slices, slice_width, trunk_crown_sep) = {
		let slice_width = 0.05;

		let slices = ((height / slice_width).ceil() as usize) + 1;
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
			variance[i] /= (means[i].1 as f32).sqrt();
			if variance[i] > max_var {
				max_var = variance[i];
			}
		}
		let mut mapped = vec![0; slices];
		for i in 0..variance.len() {
			let percent = variance[i] / max_var;
			mapped[i] = map_to_u32(percent);
		}

		let sep = mapped
			.iter()
			.enumerate()
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
				let mut mean = Vector::<3, f32>::new([0.0, 0.0, 0.0]);
				for entry in neighbors {
					mean += data[entry.index];
				}
				mean / neighbors.len() as f32
			};
			let variance = {
				let mut variance = Mat::<3, f32>::default();
				for entry in neighbors {
					let difference = data[entry.index] - mean;
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

			let size = neighbors[1..]
				.iter()
				.copied()
				.map(|entry| entry.distance.sqrt())
				.sum::<f32>();
			let size = size / (neighbors.len() - 1) as f32 / 2.0;

			Point {
				render: render::Point {
					position: data[i],
					normal: eigen_vectors[Z],
					size,
				},
				segment,
				slice: slices[((data[i][Y] - min) / slice_width) as usize],
				sub_index: ((data[i][Y] - min) / (max - min) * u32::MAX as f32) as u32,
				curve: map_to_u32((3.0 * eigen_values[Z]) / (eigen_values[X] + eigen_values[Y] + eigen_values[Z])),
			}
		})
		.collect::<Vec<Point>>();

	let trunk_heigth = trunk_crown_sep - min;
	let crown_heigth = max - trunk_crown_sep;
	(
		res,
		SegmentInformation {
			trunk_height: common::Value::RelativeHeight {
				absolute: trunk_heigth,
				percent: trunk_heigth / height,
			},
			crown_height: common::Value::RelativeHeight {
				absolute: crown_heigth,
				percent: crown_heigth / height,
			},
		},
	)
}


pub struct Adapter;


impl k_nearest::Adapter<3, f32, Vector<3, f32>> for Adapter {
	fn get(point: &Vector<3, f32>, dimension: Dimension) -> f32 {
		point[dimension]
	}


	fn get_all(point: &Vector<3, f32>) -> [f32; 3] {
		point.data()
	}
}


pub struct NeighborsTree {
	tree: k_nearest::KDTree<3, f32, Vector<3, f32>, Adapter, k_nearest::EuclideanDistanceSquared>,
}


impl NeighborsTree {
	pub fn new(points: &[Vector<3, f32>]) -> Self {
		let tree = <k_nearest::KDTree<3, f32, Vector<3, f32>, Adapter, k_nearest::EuclideanDistanceSquared>>::new(points);

		Self { tree }
	}


	pub fn get<'a>(
		&self,
		index: usize,
		data: &[Vector<3, f32>],
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
