use nalgebra as na;
use std::{
	collections::HashMap,
	path::PathBuf,
	sync::{
		atomic::{AtomicBool, Ordering},
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
	done: AtomicBool,
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
			done: AtomicBool::new(false),
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
				})
				.unwrap();
				loading.done.store(true, Ordering::Relaxed);
			});
		}
		(loading, receiver)
	}

	pub fn ui(&self, ui: &mut egui::Ui, display_settings: &mut DisplaySettings) -> LoadingResponse {
		let mut response = LoadingResponse::None;
		if ui.button("Close").clicked() {
			response = LoadingResponse::Close;
		}

		let background = &mut display_settings.background.coords.data.0[0];
		egui::color_picker::color_edit_button_rgb(ui, background);

		let popup_response = ui.button("Point size");
		let popup_id = ui.make_persistent_id("point size popup");
		if popup_response.clicked() {
			ui.memory_mut(|mem| mem.toggle_popup(popup_id));
		}
		egui::popup::popup_below_widget(ui, popup_id, &popup_response, |ui| {
			ui.set_min_width(200.0);
			if ui
				.add(
					egui::Slider::new(
						&mut display_settings.point_cloud_environment.scale,
						0.005..=2.0,
					)
					.logarithmic(true),
				)
				.changed()
			{
				display_settings.point_cloud_environment.update(&self.state);
			}
		});

		if self.done.load(Ordering::Relaxed) && ui.button("Continue").clicked() {
			self.sender.send(Event::Done).unwrap();
		}

		response
	}
}

pub enum LoadingResponse {
	None,
	Close,
}
