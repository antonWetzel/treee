use crate::{environment, program::Event};

pub struct Empty {
	sender: crossbeam::channel::Sender<Event>,
}

impl Empty {
	pub fn new() -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();
		_ = sender.send(Event::ClearPointClouds);
		(Self { sender }, receiver)
	}

	pub fn ui(&self, ui: &mut egui::Ui) {
		if ui
			.add_sized([ui.available_width(), 0.0], egui::Button::new("Load"))
			.clicked()
		{
			environment::Source::new(&self.sender);
		}
	}
}
