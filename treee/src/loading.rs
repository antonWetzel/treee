use nalgebra as na;
use std::{
	collections::HashMap,
	path::PathBuf,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc, Mutex,
	},
};

use crate::{laz::Laz, program::Event};

#[derive(Debug)]
pub struct Loading {
	pub min: na::Point3<f32>,
	pub max: na::Point3<f32>,
	sender: crossbeam::channel::Sender<Event>,
	pub total: usize,
	pub shared: Arc<Shared>,
}

#[derive(Debug)]
pub struct Shared {
	progress: AtomicUsize,
	pub state: Arc<render::State>,
	pub world_offset: na::Point3<f64>,

	pub chunks: Mutex<HashMap<usize, LoadingChunk>>,
	pub slices: Mutex<HashMap<isize, Vec<na::Point3<f32>>>>,
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
	pub fn new(path: PathBuf, state: Arc<render::State>) -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();
		sender
			.send(Event::Lookup(render::Lookup::new_png(
				&state,
				include_bytes!("../assets/white.png"),
				u32::MAX,
			)))
			.unwrap();
		let point_clouds = Mutex::new(HashMap::new());

		let laz = Laz::new(&path, None).unwrap();
		let (min, max) = (laz.min, laz.max);
		let total = laz.total();

		let shared = Shared {
			state: state.clone(),
			slices: Mutex::new(HashMap::new()),
			chunks: point_clouds,
			progress: AtomicUsize::new(0),
			world_offset: laz.world_offset,
		};
		let shared = Arc::new(shared);

		spawn_load_worker(laz, shared.clone());

		let loading = Self { sender, min, max, total, shared };

		(loading, receiver)
	}

	pub fn ui(&mut self, ui: &mut egui::Ui) {
		ui.separator();
		let progress = self.shared.progress.load(Ordering::Relaxed);
		if progress < self.total {
			let progress = progress as f32 / self.total as f32;
			ui.add(egui::ProgressBar::new(progress).rounding(egui::Rounding::ZERO));
		} else {
			if ui
				.add_sized([ui.available_width(), 0.0], egui::Button::new("Add"))
				.clicked()
			{
				let path = rfd::FileDialog::new()
					.set_title("Load")
					.add_filter("Pointcloud", &["las", "laz"])
					.pick_file();
				if let Some(path) = path {
					let laz = Laz::new(&path, Some(self.shared.world_offset)).unwrap();
					for dim in 0..3 {
						self.min[dim] = self.min[dim].min(laz.min[dim]);
						self.max[dim] = self.max[dim].max(laz.max[dim]);
					}
					self.total = laz.total();
					self.shared.progress.store(0, Ordering::Relaxed);
					spawn_load_worker(laz, self.shared.clone());
				}
			}

			if ui
				.add_sized([ui.available_width(), 0.0], egui::Button::new("Continue"))
				.clicked()
			{
				self.sender.send(Event::Done).unwrap();
			}
		}
	}
}

fn spawn_load_worker(laz: Laz, shared: Arc<Shared>) {
	std::thread::spawn(move || {
		laz.read(|chunk| {
			let points = chunk.read();

			let segment = LoadingChunk::new(&points, &shared.state);
			let mut point_clouds = shared.chunks.lock().unwrap();
			let mut idx = rand::random();
			while point_clouds.contains_key(&idx) {
				idx = rand::random();
			}
			point_clouds.insert(idx, segment);
			drop(point_clouds);

			let mut slices = shared.slices.lock().unwrap();
			for p in points {
				let idx = p.y.floor() as isize;
				slices.entry(idx).or_default().push(p);
			}
			drop(slices);
			shared.progress.fetch_add(1, Ordering::Relaxed);
		})
		.unwrap();
	});
}
