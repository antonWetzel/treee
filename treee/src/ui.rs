use std::path::PathBuf;

use crate::program::{DisplaySettings, Preview};

pub enum EmptyResponse {
	None,
	Load(PathBuf),
}

pub fn empty(ui: &mut egui::Ui) -> EmptyResponse {
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

pub enum PreviewResponse {
	None,
	Close,
}

pub fn preview(ui: &mut egui::Ui, display_settings: &mut DisplaySettings, preview: &Preview) -> PreviewResponse {
	let mut response = PreviewResponse::None;
	if ui.button("Close").clicked() {
		response = PreviewResponse::Close;
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
			display_settings
				.point_cloud_environment
				.update(&preview.state);
		}
	});

	ui.add_space(ui.available_width() - 100.0);
	response
}
