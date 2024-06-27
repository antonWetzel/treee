use crate::{program::DisplaySettings, task::Task};

pub fn ui(
	ui: &mut egui::Ui,
	display_settings: &mut DisplaySettings,
	injector: &crossbeam::deque::Injector<Task>,
	state: &render::State,
) {
	let background = &mut display_settings.background.coords.data.0[0];
	egui::color_picker::color_edit_button_rgb(ui, background);

	if ui.button("Load").clicked() {
		let path = rfd::FileDialog::new()
			.set_title("Load")
			.add_filter("Pointcloud", &["las", "laz"])
			.pick_file();
		if let Some(path) = path {
			injector.push(Task::Load(path));
		}
	}

	// if ui.button("Update").clicked() {
	// 	injector.push(Task::Update);
	// }

	let response = ui.button("Point size");
	let popup_id = ui.make_persistent_id("point size popup");
	if response.clicked() {
		ui.memory_mut(|mem| mem.toggle_popup(popup_id));
	}
	egui::popup::popup_below_widget(ui, popup_id, &response, |ui| {
		ui.set_min_width(200.0);
		if ui
			.add(egui::Slider::new(
				&mut display_settings.point_cloud_environment.scale,
				0.0..=2.0,
			))
			.changed()
		{
			display_settings.point_cloud_environment.update(state);
		}
	});
}
