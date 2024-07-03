use crate::{environment, program::Event};

pub struct Empty {
	sender: crossbeam::channel::Sender<Event>,
}

impl Empty {
	pub fn new() -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();
		sender.send(Event::ClearPointClouds).unwrap();
		(Self { sender }, receiver)
	}

	pub fn ui(&self, ui: &mut egui::Ui) {
		if ui
			.add_sized([ui.available_width(), 0.0], egui::Button::new("Load"))
			.clicked()
		{
			environment::get_source(&self.sender);
		}
	}
}
