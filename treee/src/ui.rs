use crate::DisplaySettings;

pub fn ui(ui: &mut egui::Ui, display_settings: &mut DisplaySettings) {
	let color = &mut display_settings.background.coords.data.0[0];
	egui::color_picker::color_edit_button_rgb(ui, color);
}
