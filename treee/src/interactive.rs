use std::sync::Arc;

use crate::program::Event;

pub struct Interactive {}

impl Interactive {
	pub fn new() -> (Arc<Self>, crossbeam::channel::Receiver<Event>) {
		let (_, receiver) = crossbeam::channel::bounded(0);
		let interactive = Self {};
		(Arc::new(interactive), receiver)
	}

	pub fn ui(&self, ui: &mut egui::Ui) -> InteractiveResponse {
		let mut response = InteractiveResponse::None;

		response
	}
}
pub enum InteractiveResponse {
	None,
}
