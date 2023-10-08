pub struct Interface {
	statistics: render::UIElement,
}

impl Interface {
	pub fn new() -> Self {
		Self {
			statistics: render::UIElement::new("...", [10.0, 10.0].into(), 25.0),
		}
	}

	pub fn update_statisitics(&mut self, fps: usize, workload: usize) {
		self.statistics.text = format!("FPS: {} | Chunks queued: {}", fps, workload);
	}
}
impl render::UICollect for Interface {
	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		collector.add_element(&self.statistics);
	}
}
