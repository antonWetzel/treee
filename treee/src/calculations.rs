use core::f32;
use std::{
	collections::HashMap,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
};

use crossbeam::channel::TrySendError;
use nalgebra as na;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{program::Event, segmenting::Tree};

/// Slice width for calculations.
const SLICE_WIDTH: f32 = 0.1;

/// State for the Calculations phase.
pub struct Calculations {
	pub shared: Arc<Shared>,
	pub total: usize,
	pub world_offset: na::Point3<f64>,
}

/// Shared state for the workers.
#[derive(Debug)]
pub struct Shared {
	pub segments: std::sync::Mutex<HashMap<u32, SegmentData>>,
	pub progress: AtomicUsize,
}

/// Data for a segment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SegmentData {
	pub points: Vec<na::Point3<f32>>,
	pub classifications: Vec<Classification>,
	pub info: SegmentInformation,

	pub min: na::Point3<f32>,
	pub max: na::Point3<f32>,
	pub coords: Option<(f64, f64)>,
}

/// Classification for a point.
#[derive(
	Debug,
	Clone,
	Copy,
	serde::Serialize,
	serde::Deserialize,
	PartialEq,
	Eq
)]
pub enum Classification {
	Ground,
	Trunk,
	Crown,
}

/// Calculated information to save for one segment.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SegmentSave {
	#[serde(flatten)]
	pub info: SegmentInformation,
	pub min: na::Point3<f32>,
	pub max: na::Point3<f32>,
	pub offset: na::Point3<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub latitude: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub longitude: Option<f64>,
}

/// Limit event queue size.
const SENDER_CAPACITY: usize = 128;

impl Calculations {
	/// Create phase from segments.
	pub fn new(
		segments: HashMap<u32, Vec<na::Point3<f32>>>,
		world_offset: na::Point3<f64>,
	) -> (Self, crossbeam::channel::Receiver<Event>) {
		let shared = Shared {
			segments: std::sync::Mutex::new(HashMap::new()),
			progress: AtomicUsize::new(0),
		};
		let shared = Arc::new(shared);
		let total = segments.len();

		let (sender, reciever) = crossbeam::channel::bounded(SENDER_CAPACITY);
		_ = sender.send(Event::ClearPointClouds);

		{
			let shared = shared.clone();
			rayon::spawn(move || {
				segments.into_par_iter().for_each(|(idx, points)| {
					let seg = SegmentData::new(points);

					while sender.len() > SENDER_CAPACITY - 16 {
						std::hint::spin_loop();
					}
					let mut event = Event::PointCloud {
						idx: Some(idx),
						data: seg.points.clone(),
						segment: vec![idx; seg.points.len()],
					};
					loop {
						match sender.try_send(event) {
							Ok(_) => break,
							Err(TrySendError::Disconnected(_)) => break,
							Err(TrySendError::Full(v)) => event = v,
						}
					}

					shared.segments.lock().unwrap().insert(idx, seg);
					shared.progress.fetch_add(1, Ordering::Relaxed);
				});
				_ = sender.send(Event::Done);
			});
		}

		(Self { shared, total, world_offset }, reciever)
	}

	/// Draw UI.
	pub fn ui(&mut self, ui: &mut egui::Ui) {
		let progress = self.shared.progress.load(Ordering::Relaxed) as f32 / self.total as f32;
		ui.add(egui::ProgressBar::new(progress).rounding(egui::Rounding::ZERO));
	}
}

impl SegmentData {
	/// Create segment from points.
	pub fn new(points: Vec<na::Point3<f32>>) -> Self {
		let (min, max) = if points.is_empty() {
			(na::point![0.0, 0.0, 0.0], na::point![0.0, 0.0, 0.0])
		} else {
			let (mut min, mut max) = (points[0], points[0]);
			for p in points.iter() {
				for dim in 0..3 {
					min[dim] = min[dim].min(p[dim]);
					max[dim] = max[dim].max(p[dim]);
				}
			}
			(min, max)
		};

		let info = SegmentInformation::new(&points, min.y, max.y);
		let crown_sep = max.y - info.crown_height;
		let ground_sep = crown_sep - info.trunk_height;
		let classifications = points
			.iter()
			.map(|&p| {
				if p.y < ground_sep {
					Classification::Ground
				} else if p.y < crown_sep {
					Classification::Trunk
				} else {
					Classification::Crown
				}
			})
			.collect();

		Self {
			points,
			classifications,
			info,
			min,
			max,
			coords: None,
		}
	}

	/// Update the render data for the segment.
	pub fn update_render(&self, idx: u32, sender: &crossbeam::channel::Sender<Event>) {
		if self.points.is_empty() {
			_ = sender.send(Event::RemovePointCloud(idx));
			return;
		}
		_ = sender.send(Event::PointCloud {
			idx: Some(idx),
			data: self.points.clone(),
			segment: vec![idx; self.points.len()],
		});
	}
}

/// Calculated information for a segment.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SegmentInformation {
	pub trunk_height: f32,
	pub crown_height: f32,

	pub ground_sep: f32,
	pub crown_sep: f32,
}

impl SegmentInformation {
	/// Calculate from points.
	pub fn new(data: &[na::Point3<f32>], min: f32, max: f32) -> Self {
		let height = max - min;

		let ground_max_search_height = 1.0;
		let ground_min_area_scale = 1.5;
		let min_crown_diameter = 2.0f32;

		let slices = ((height / SLICE_WIDTH) as usize) + 1;
		let mut sets = vec![<Option<Tree>>::None; slices];
		for pos in data.iter().copied() {
			let idx = ((pos.y - min) / SLICE_WIDTH) as usize;
			match &mut sets[idx] {
				Some(tree) => tree.insert(na::vector![pos.x, pos.z].into()),
				x @ None => *x = Some(Tree::new(na::vector![pos.x, pos.z].into())),
			}
		}

		let areas = get_size_areas(min, height, data, |_| true);

		let min_area = areas
			.iter()
			.copied()
			.skip((1.0 / SLICE_WIDTH) as usize)
			.take((10.0 / SLICE_WIDTH) as usize)
			.min_by(|a, b| a.total_cmp(b))
			.unwrap_or(0.5)
			.max(0.5);
		let ground = areas
			.iter()
			.copied()
			.enumerate()
			.take((ground_max_search_height / SLICE_WIDTH) as usize)
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

		let min_crown_area = std::f32::consts::PI * (min_crown_diameter / 2.0).powi(2);

		let crown_sep = areas
			.iter()
			.enumerate()
			.skip(ground_sep)
			.find(|&(_, &v)| v > min_crown_area)
			.map(|(index, _)| index)
			.unwrap_or(0);

		let ground_sep = min + ground_sep as f32 * SLICE_WIDTH;
		let crown_sep = min + crown_sep as f32 * SLICE_WIDTH;

		Self {
			trunk_height: crown_sep - ground_sep,
			crown_height: max - crown_sep,
			ground_sep,
			crown_sep,
		}
	}

	/// Recalculate the information.
	pub fn update(
		&mut self,
		data: &[na::Point3<f32>],
		classifications: &[Classification],
		min: f32,
		max: f32,
		calc_curve: bool,
	) -> CalculationProperties {
		let neighbors_count = 31;
		let neighbors_max_distance = f32::MAX;

		let height = max - min;

		let slices = ((height / SLICE_WIDTH) as usize) + 1;
		let mut sets = vec![<Option<Tree>>::None; slices];
		for pos in data
			.iter()
			.zip(classifications)
			.filter_map(|(&p, &c)| (c == Classification::Crown).then_some(p))
		{
			let idx = ((pos.y - min) / SLICE_WIDTH) as usize;
			match &mut sets[idx] {
				Some(tree) => tree.insert(na::vector![pos.x, pos.z].into()),
				x @ None => *x = Some(Tree::new(na::vector![pos.x, pos.z].into())),
			}
		}

		let areas = get_size_areas(min, height, data, |idx| {
			classifications[idx] == Classification::Crown
		});

		let crown_area = areas
			.iter()
			.copied()
			.skip(((self.crown_sep - min) / SLICE_WIDTH) as usize)
			.max_by(|a, b| a.total_cmp(b))
			.unwrap_or(0.0);

		let crown_diameter = approximate_diameter(crown_area);
		let slices = areas
			.into_iter()
			.map(|area| approximate_diameter(area) / crown_diameter)
			.collect::<Vec<_>>();

		let expansion = data
			.iter()
			.copied()
			.map(|p| {
				let idx = ((p.y - min) / SLICE_WIDTH) as usize;
				slices[idx]
			})
			.collect();

		let height = data
			.iter()
			.map(|p| (p.y - min) / height)
			.collect::<Vec<_>>();

		let curve = if calc_curve {
			let neighbors_tree = NeighborsTree::new(data);

			let mut neighbors_location = bytemuck::zeroed_vec(neighbors_count);
			data.iter()
				.enumerate()
				.map(|(i, _)| {
					let neighbors = neighbors_tree.get(
						i,
						data,
						&mut neighbors_location,
						neighbors_max_distance,
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
					(3.0 * eigen_values.z) / (eigen_values.x + eigen_values.y + eigen_values.z)
				})
				.collect()
		} else {
			vec![0.0; data.len()]
		};

		CalculationProperties { expansion, curve, height }
	}
}

/// Adapter to use generic KD-Tree.
pub struct Adapter;
impl k_nearest::Adapter<3, f32, na::Point3<f32>> for Adapter {
	fn get(point: &na::Point3<f32>, dimension: usize) -> f32 {
		point[dimension]
	}

	fn get_all(point: &na::Point3<f32>) -> [f32; 3] {
		point.coords.data.0[0]
	}
}

/// Wrapper for KD-Tree
pub struct NeighborsTree {
	tree: k_nearest::KDTree<3, f32, na::Point3<f32>, Adapter, k_nearest::EuclideanDistanceSquared>,
}

impl NeighborsTree {
	pub fn new(points: &[na::Point3<f32>]) -> Self {
		let tree = <k_nearest::KDTree<
			3,
			f32,
			na::Point3<f32>,
			Adapter,
			k_nearest::EuclideanDistanceSquared,
		>>::new(points);

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

/// Calculated properties for one segment.
#[derive(Debug, Clone)]
pub struct CalculationProperties {
	pub expansion: Vec<f32>,
	pub curve: Vec<f32>,
	pub height: Vec<f32>,
}

/// Seperate points into slices and calculate convex areas.
pub fn get_size_areas(
	min: f32,
	height: f32,
	data: &[na::Point3<f32>],
	valid: impl Fn(usize) -> bool,
) -> Vec<f32> {
	let slices = ((height / SLICE_WIDTH) as usize) + 1;
	let mut sets = vec![<Option<Tree>>::None; slices];
	for pos in data
		.iter()
		.enumerate()
		.filter_map(|(idx, &p)| valid(idx).then_some(p))
	{
		let idx = ((pos.y - min) / SLICE_WIDTH) as usize;
		match &mut sets[idx] {
			Some(tree) => tree.insert(na::vector![pos.x, pos.z].into()),
			x @ None => *x = Some(Tree::new(na::vector![pos.x, pos.z].into())),
		}
	}
	sets.into_iter()
		.map(|set| set.map(|set| set.statistics().area).unwrap_or(0.0))
		.collect::<Vec<_>>()
}

/// Convert value in range [0.0, 1.0] to [0, u32::MAX]
pub fn map_to_u32(value: f32) -> u32 {
	(value * u32::MAX as f32) as u32
}

/// Calculate circum-sphere for the points.
/// Returns `None` if the points are close to linear.
///
/// Source: https://stackoverflow.com/a/34326390
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
	let to = (na::vector![-ab.y, ab.x] * ac.norm_squared()
		+ na::vector![ac.y, -ac.x] * ab.norm_squared())
		/ (2.0 * cross);
	let radius = to.norm();
	if radius.is_nan() {
		return None;
	}
	Some((point_a + to, radius))
}

/// Approxiamte diameter based on the area.
fn approximate_diameter(area: f32) -> f32 {
	2.0 * (area / std::f32::consts::PI).sqrt()
}

/// Calculate eigenvalues for a real and symmetric 3x3 matrix.
///
/// Source: https://en.wikipedia.org/wiki/Eigenvalue_algorithm#3%C3%973_matrices
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
