use std::path::PathBuf;

use crate::program::Event;

pub struct Empty;

impl Empty {
	pub fn new() -> (Self, crossbeam::channel::Receiver<Event>) {
		let (_, receiver) = crossbeam::channel::bounded(0);
		(Self, receiver)
	}

	pub fn ui(&self, ui: &mut egui::Ui) -> EmptyResponse {
		let mut response = EmptyResponse::None;
		if ui.button("Load").clicked() {
			let path = rfd::FileDialog::new()
				.set_title("Load")
				.add_filter("Pointcloud", &["las", "laz"])
				.pick_file();
			if let Some(path) = path {
				response = EmptyResponse::Load(path);
			}
		}
		response
	}
}
pub enum EmptyResponse {
	None,
	Load(PathBuf),
}
