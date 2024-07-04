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

use crate::{interactive::DELETED_INDEX, program::Event, segmenting::Tree};

pub struct Calculations {
	pub display: DisplayModus,
	pub shared: Arc<Shared>,
	pub total: usize,
	pub world_offset: na::Point3<f64>,
}

#[derive(Debug, Clone, Copy)]
pub enum DisplayModus {
	Segment,
	Property,
}

impl DisplayModus {
	pub fn ui(&mut self, ui: &mut egui::Ui) {
		ui.separator();
		ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Display"));
		if ui
			.radio(matches!(self, DisplayModus::Segment), "Segment")
			.clicked()
		{
			*self = DisplayModus::Segment;
		}
		if ui
			.radio(matches!(self, DisplayModus::Property), "Classification")
			.clicked()
		{
			*self = DisplayModus::Property;
		}
	}
}

#[derive(Debug)]
pub struct Shared {
	pub segments: std::sync::Mutex<HashMap<u32, SegmentData>>,
	pub progress: AtomicUsize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SegmentData {
	pub points: Vec<na::Point3<f32>>,
	pub info: SegmentInformation,
	pub min: na::Point3<f32>,
	pub max: na::Point3<f32>,
	pub coords: Option<(f64, f64)>,
}

const SENDER_CAPACITY: usize = 128;

impl Calculations {
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

		_ = sender.send(Event::Lookup {
			bytes: include_bytes!("../assets/grad_turbo.png"),
			max: u32::MAX,
		});

		{
			let shared = shared.clone();
			rayon::spawn(move || {
				let mut segs = HashMap::<u32, _>::new();
				for (_, points) in segments.into_iter() {
					let mut idx = rand::random();
					while idx == DELETED_INDEX || segs.contains_key(&idx) {
						idx = rand::random();
					}
					segs.insert(idx, points);
				}
				segs.into_par_iter().for_each(|(idx, points)| {
					let seg = SegmentData::new(points);

					while sender.len() > SENDER_CAPACITY - 16 {
						std::hint::spin_loop();
					}
					let mut event = Event::PointCloud {
						idx: Some(idx),
						data: seg.points.clone(),
						segment: vec![idx as u32; seg.points.len()],
						property: Some(seg.property()),
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

		(
			Self {
				shared,
				display: DisplayModus::Segment,
				total,
				world_offset,
			},
			reciever,
		)
	}

	pub fn ui(&mut self, ui: &mut egui::Ui) {
		self.display.ui(ui);

		ui.separator();
		let progress = self.shared.progress.load(Ordering::Relaxed) as f32 / self.total as f32;
		ui.add(egui::ProgressBar::new(progress).rounding(egui::Rounding::ZERO));
	}
}

impl SegmentData {
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
		Self { points, info, min, max, coords: None }
	}

	pub fn property(&self) -> Vec<u32> {
		let mut property = vec![0u32; self.points.len()];
		for (idx, p) in self.points.iter().enumerate() {
			property[idx] = if p.y < self.info.ground_sep {
				u32::MAX / 8 * 1
			} else if p.y < self.info.crown_sep {
				u32::MAX / 8 * 3
			} else {
				u32::MAX / 8 * 6
			};
		}
		property
	}

	pub fn update_render(&self, idx: u32, sender: &crossbeam::channel::Sender<Event>) {
		if self.points.is_empty() {
			_ = sender.send(Event::RemovePointCloud(idx));
			return;
		}
		_ = sender.send(Event::PointCloud {
			idx: Some(idx),
			data: self.points.clone(),
			segment: vec![idx as u32; self.points.len()],
			property: Some(self.property()),
		});
	}
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
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
			let trunk_min = ground_sep as f32 * slice_width + trunk_diameter_height
				- 0.5 * trunk_diameter_range;
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

		let min_crown_area =
			std::f32::consts::PI * ((trunk_diameter + crown_diameter_difference) / 2.0).powi(2);

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

	pub fn redo_diameters(&mut self, data: &[na::Point3<f32>], min: f32, max: f32) {
		let height = max - min;

		let slice_width = 0.1;
		let trunk_diameter_range = 0.2;

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

		let trunk_diameter = {
			let trunk_min = self.ground_sep - 0.5 * trunk_diameter_range;
			let trunk_max = trunk_min + trunk_diameter_range;
			let slice_trunk = data
				.iter()
				.filter(|p| (trunk_min..trunk_max).contains(&(p.y)))
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

			best_circle.0
		};

		let crown_area = areas
			.iter()
			.copied()
			.skip(((self.crown_sep - min) / slice_width) as usize)
			.max_by(|a, b| a.total_cmp(b))
			.unwrap_or(0.0);
		self.trunk_diameter = trunk_diameter;
		self.crown_diameter = approximate_diameter(crown_area);
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
	let to = (na::vector![-ab.y, ab.x] * ac.norm_squared()
		+ na::vector![ac.y, -ac.x] * ab.norm_squared())
		/ (2.0 * cross);
	let radius = to.norm();
	if radius.is_nan() {
		return None;
	}
	Some((point_a + to, radius))
}

fn approximate_diameter(area: f32) -> f32 {
	2.0 * (area / std::f32::consts::PI).sqrt()
}
