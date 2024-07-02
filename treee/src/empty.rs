use crate::program::Event;

pub struct Empty {
	sender: crossbeam::channel::Sender<Event>,
}

impl Empty {
	pub fn new() -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();
		(Self { sender }, receiver)
	}

	pub fn ui(&self, ui: &mut egui::Ui) {
		if ui
			.add_sized([ui.available_width(), 0.0], egui::Button::new("Load"))
			.clicked()
		{
			let path = rfd::FileDialog::new()
				.set_title("Load")
				.add_filter("Pointcloud", &["las", "laz", "ipc"])
				.pick_file();
			if let Some(path) = path {
				self.sender.send(Event::Load(path)).unwrap();
			}
		}
	}
}
