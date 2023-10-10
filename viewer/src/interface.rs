pub struct Interface {
	last_fps: usize,
	last_workload: usize,
	statistics: render::UIElement,
}

impl Interface {
	pub fn new() -> Self {
		Self {
			last_fps: 0,
			last_workload: 0,
			statistics: render::UIElement::new(
				vec![
					"...\n".into(),
					"...\n".into(),
					"...\n".into(),
					"...\n".into(),
				],
				[10.0, 10.0].into(),
				25.0,
			),
		}
	}

	pub fn update_fps(&mut self, fps: usize) {
		if fps != self.last_fps {
			self.statistics.text[0] = format!("FPS: {}\n", fps);
			self.last_fps = fps;
		}
	}

	pub fn update_workload(&mut self, workload: usize) {
		if workload != self.last_workload {
			self.statistics.text[1] = format!("Chunks queued: {}\n", workload);
			self.last_workload = workload;
		}
	}

	pub fn update_eye_dome_settings(&mut self, strength: f32, sensitivity: f32) {
		self.statistics.text[2] = format!("Highlight Strength: {}\n", strength);
		self.statistics.text[3] = format!("Highlight Sensitivity: {}\n", sensitivity);
	}
}

impl render::UICollect for Interface {
	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		collector.add_element(&self.statistics);
	}
}
