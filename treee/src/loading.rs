use std::{
	collections::HashMap,
	path::PathBuf,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
};

use crate::{
	laz::Laz,
	octree::{Octree, MAX_POINTS},
	program::{DisplaySettings, Event},
};

#[derive(Debug)]
pub struct Loading {
	pub state: Arc<render::State>,
	pub octree: Octree,

	pub point_clouds: std::sync::Mutex<HashMap<usize, render::PointCloud>>,
	pub property: render::PointCloudProperty,
	sender: crossbeam::channel::Sender<Event>,
	done: AtomicBool,
}

impl Loading {
	pub fn new(path: PathBuf, state: Arc<render::State>) -> (Arc<Self>, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();
		let point_clouds = std::sync::Mutex::new(HashMap::new());

		let laz = Laz::new(&path).unwrap();

		let loading = Self {
			property: render::PointCloudProperty::new_empty(&state, MAX_POINTS),
			state: state.clone(),
			octree: Octree::new(laz.min, laz.max),
			sender,

			point_clouds,
			done: AtomicBool::new(false),
		};
		let loading = Arc::new(loading);

		{
			let (sender, receiver) = crossbeam::channel::unbounded();
			let loading = loading.clone();
			std::thread::spawn(move || {
				laz.read(|chunk| {
					while receiver.len() > 1000 {
						let Ok(idx) = receiver.try_recv() else {
							break;
						};
						loading
							.octree
							.update(&loading.state, &loading.point_clouds, idx);
					}
					loading
						.octree
						.insert(chunk.read(), |idx| sender.send(idx).unwrap());
				})
				.unwrap();
				drop(sender);
				while let Ok(idx) = receiver.recv() {
					loading
						.octree
						.update(&loading.state, &loading.point_clouds, idx);
				}
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

		if self.done.load(Ordering::Relaxed) {
			if ui.button("Continue").clicked() {
				self.sender.send(Event::Done).unwrap();
			}
		}

		response
	}
}

pub enum LoadingResponse {
	None,
	Close,
}
