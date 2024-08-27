use crossbeam::channel::SendError;
use nalgebra as na;
use std::{
	collections::HashMap,
	ops::Not,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc, Mutex,
	},
};

use crate::{environment, laz::Laz, program::Event, Error};

#[derive(Debug)]
pub struct Loading {
	pub min: na::Point3<f32>,
	pub max: na::Point3<f32>,
	pub total: usize,
	pub shared: Arc<Shared>,
}

#[derive(Debug)]
pub struct Shared {
	progress: AtomicUsize,
	sender: crossbeam::channel::Sender<Event>,
	pub world_offset: na::Point3<f64>,
	pub slices: Mutex<HashMap<isize, Vec<na::Point3<f32>>>>,
}

impl Loading {
	pub fn new(source: environment::Source) -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::bounded(8);
		_ = sender.send(Event::Lookup {
			bytes: include_bytes!("../assets/white.png"),
			max: u32::MAX,
		});

		let laz = Laz::new(source, None).unwrap();
		let (min, max) = (laz.min, laz.max);
		let total = laz.total();

		let shared = Shared {
			slices: Mutex::new(HashMap::new()),
			progress: AtomicUsize::new(0),
			world_offset: laz.world_offset,
			sender,
		};
		let shared = Arc::new(shared);

		spawn_load_worker(laz, shared.clone());

		_ = shared.sender.send(Event::ClearPointClouds);
		let loading = Self { min, max, total, shared };

		(loading, receiver)
	}

	pub fn ui(&mut self, ui: &mut egui::Ui) {
		ui.separator();
		let progress = self.shared.progress.load(Ordering::Relaxed);
		if progress < self.total {
			let progress = progress as f32 / self.total as f32;
			ui.add(egui::ProgressBar::new(progress).rounding(egui::Rounding::ZERO));
		} else {
			#[cfg(not(target_arch = "wasm32"))]
			if ui
				.add_sized([ui.available_width(), 0.0], egui::Button::new("Add"))
				.clicked()
			{
				environment::Source::new(&self.shared.sender);
			}

			if ui
				.add_sized([ui.available_width(), 0.0], egui::Button::new("Continue"))
				.clicked()
			{
				_ = self.shared.sender.send(Event::Done);
			}
		}
	}

	pub fn add(&mut self, source: environment::Source) {
		let laz = Laz::new(source, Some(self.shared.world_offset)).unwrap();
		for dim in 0..3 {
			self.min[dim] = self.min[dim].min(laz.min[dim]);
			self.max[dim] = self.max[dim].max(laz.max[dim]);
		}
		self.total = laz.total();
		self.shared.progress.store(0, Ordering::Relaxed);
		spawn_load_worker(laz, self.shared.clone());
	}
}

fn spawn_load_worker(laz: Laz, shared: Arc<Shared>) {
	rayon::spawn(move || {
		_ = laz.read(|chunk| {
			let points = chunk.read();

			if points.is_empty().not() {
				let mut slices = shared.slices.lock().unwrap();
				for &p in points.iter() {
					let idx = p.y.floor() as isize;
					slices.entry(idx).or_default().push(p);
				}
				drop(slices);

				let segment = vec![0; points.len()];
				shared
					.sender
					.send(Event::PointCloud { idx: None, data: points, segment })
					.map_err(|_| Error::CorruptFile)?;
			}

			shared.progress.fetch_add(1, Ordering::Relaxed);
			Ok(())
		});
	});
}
