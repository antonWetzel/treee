use nalgebra as na;
use std::{
	collections::HashMap,
	path::PathBuf,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc, Mutex,
	},
};

use crate::{
	laz::Laz,
	program::{DisplaySettings, Event},
};

#[derive(Debug)]
pub struct Loading {
	pub state: Arc<render::State>,

	pub chunks: Mutex<HashMap<usize, LoadingChunk>>,
	pub slices: Mutex<Vec<Vec<na::Point3<f32>>>>,
	pub min: na::Point3<f32>,
	pub max: na::Point3<f32>,
	sender: crossbeam::channel::Sender<Event>,
	pub total: usize,
	progress: AtomicUsize,
	pub world_offset: na::Point3<f64>,
}

#[derive(Debug)]
pub struct LoadingChunk {
	pub point_cloud: render::PointCloud,
	pub property: render::PointCloudProperty,
}

impl LoadingChunk {
	pub fn new(points: &[na::Point3<f32>], state: &render::State) -> Self {
		let point_cloud = render::PointCloud::new(state, points);
		let property = render::PointCloudProperty::new(state, &vec![0; points.len()]);
		Self { point_cloud, property }
	}
}

impl Loading {
	pub fn new(path: PathBuf, state: Arc<render::State>) -> (Arc<Self>, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();
		sender
			.send(Event::Lookup(render::Lookup::new_png(
				&state,
				include_bytes!("../assets/white.png"),
				u32::MAX,
			)))
			.unwrap();
		let point_clouds = Mutex::new(HashMap::new());

		let laz = Laz::new(&path).unwrap();
		let min = laz.min.y - 1.0;
		let size = laz.max.y + 2.0 - min;
		let layers = size.ceil() as usize;
		let slices = vec![Vec::new(); layers];

		let loading = Self {
			state: state.clone(),
			sender,
			min: laz.min,
			max: laz.max,
			slices: Mutex::new(slices),

			chunks: point_clouds,
			total: laz.total(),
			progress: AtomicUsize::new(0),
			world_offset: laz.world_offset,
		};
		let loading = Arc::new(loading);

		{
			let loading = loading.clone();
			std::thread::spawn(move || {
				laz.read(|chunk| {
					let points = chunk.read();

					let segment = LoadingChunk::new(&points, &loading.state);
					let mut point_clouds = loading.chunks.lock().unwrap();
					let mut idx = rand::random();
					while point_clouds.contains_key(&idx) {
						idx = rand::random();
					}
					point_clouds.insert(idx, segment);
					drop(point_clouds);

					let mut slices = loading.slices.lock().unwrap();
					for p in points {
						let idx = (p.y - loading.min.y).floor() as usize;
						slices[idx].push(p);
					}
					drop(slices);
					loading.progress.fetch_add(1, Ordering::Relaxed);
				})
				.unwrap();
			});
		}
		(loading, receiver)
	}

	pub fn ui(&self, ui: &mut egui::Ui, display_settings: &mut DisplaySettings) {
		ui.separator();
		let progress = self.progress.load(Ordering::Relaxed);
		if progress < self.total {
			let progress = progress as f32 / self.total as f32;
			ui.add(egui::ProgressBar::new(progress).rounding(egui::Rounding::ZERO));
		} else {
			if ui
				.add_sized([ui.available_width(), 0.0], egui::Button::new("Continue"))
				.clicked()
			{
				self.sender.send(Event::Done).unwrap();
			}
		}
	}
}
