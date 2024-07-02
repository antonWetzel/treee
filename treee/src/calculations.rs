use std::{
	collections::HashMap,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc,
	},
};

use dashmap::DashMap;
use nalgebra as na;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{program::Event, segmenting::Tree};

pub struct Calculations {
	pub display: DisplayModus,
	pub shared: Arc<Shared>,
	pub total: usize,
	pub world_offset: na::Point3<f64>,
}

#[derive(Debug, Clone, Copy)]
pub enum DisplayModus {
	Solid,
	Property,
}

impl DisplayModus {
	pub fn ui(&mut self, ui: &mut egui::Ui) {
		ui.separator();
		ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Display"));
		if ui
			.radio(matches!(self, DisplayModus::Solid), "Segment")
			.clicked()
		{
			*self = DisplayModus::Solid;
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
	state: Arc<render::State>,
	pub segments: std::sync::Mutex<HashMap<usize, Segment>>,
	pub progress: AtomicUsize,
}

#[derive(Debug)]
pub struct Segment {
	pub data: SegmentData,
	pub render: Option<SegmentRender>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SegmentData {
	pub points: Vec<na::Point3<f32>>,
	pub info: SegmentInformation,
	pub min: na::Point3<f32>,
	pub max: na::Point3<f32>,
	pub coords: Option<(f64, f64)>,
}

#[derive(Debug)]
pub struct SegmentRender {
	pub point_cloud: render::PointCloud,
	solid: render::PointCloudProperty,
	pub property: render::PointCloudProperty,
}

impl SegmentRender {
	pub fn new(
		points: &[na::Point3<f32>],
		idx: usize,
		info: SegmentInformation,
		state: &render::State,
	) -> Option<Self> {
		if points.is_empty() {
			return None;
		}
		let point_cloud = render::PointCloud::new(state, points);
		let solid = render::PointCloudProperty::new(state, &vec![idx as u32; points.len()]);

		let mut property = vec![0u32; points.len()];
		for (idx, p) in points.iter().enumerate() {
			property[idx] = if p.y < info.ground_sep {
				0
			} else if p.y < info.crown_sep {
				u32::MAX / 2
			} else {
				u32::MAX
			};
		}
		let property = render::PointCloudProperty::new(state, &property);
		let render = Self { point_cloud, solid, property };
		Some(render)
	}

	pub fn render<'a>(&'a self, modus: DisplayModus, point_cloud_pass: &mut render::PointCloudPass<'a>) {
		let property = match modus {
			DisplayModus::Solid => &self.solid,
			DisplayModus::Property => &self.property,
		};
		self.point_cloud.render(point_cloud_pass, property);
	}
}

impl Calculations {
	pub fn new(
		segments: DashMap<usize, Vec<na::Point3<f32>>>,
		state: Arc<render::State>,
		world_offset: na::Point3<f64>,
	) -> (Self, crossbeam::channel::Receiver<Event>) {
		let shared = Shared {
			state,
			segments: std::sync::Mutex::new(HashMap::new()),
			progress: AtomicUsize::new(0),
		};
		let shared = Arc::new(shared);
		let total = segments.len();

		let (sender, reciever) = crossbeam::channel::unbounded();

		sender
			.send(Event::Lookup(render::Lookup::new_png(
				&shared.state,
				include_bytes!("../assets/grad_turbo.png"),
				u32::MAX,
			)))
			.unwrap();

		{
			let shared = shared.clone();
			std::thread::spawn(move || {
				let mut segs = HashMap::<usize, _>::new();
				for (_, points) in segments.into_iter() {
					let mut idx = rand::random();
					while segs.contains_key(&idx) {
						idx = rand::random();
					}
					segs.insert(idx, points);
				}
				segs.into_par_iter().for_each(|(idx, points)| {
					let seg = Segment::new(points, idx, &shared.state);
					shared.segments.lock().unwrap().insert(idx, seg);
					shared.progress.fetch_add(1, Ordering::Relaxed);
				});
				sender.send(Event::Done).unwrap();
			});
		}

		(
			Self {
				shared,
				display: DisplayModus::Solid,
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

impl Segment {
	pub fn new(points: Vec<na::Point3<f32>>, idx: usize, state: &render::State) -> Self {
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
		let render = SegmentRender::new(&points, idx, info, state);
		Self {
			render,
			data: SegmentData { points, info, min, max, coords: None },
		}
	}

	pub fn from_data(idx: usize, data: SegmentData, state: &render::State) -> Self {
		let render = SegmentRender::new(&data.points, idx, data.info, state);
		Self { data, render }
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
