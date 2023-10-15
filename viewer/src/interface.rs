use math::{Vector, X, Y};

use crate::state::State;

pub struct Interface {
	pub last_workload: usize,
	pub statistics: render::UIElement,
	show_statistics: bool,
	open: Button,
	debug: Button,
}

struct Button {
	image: render::UIImage,
	position: Vector<2, f32>,
	max: Vector<2, f32>,
}

impl Button {
	pub fn new(state: &State, data: &[u8], position: Vector<2, f32>, size: Vector<2, f32>) -> Self {
		let texture = render::Texture::new(state, data, render::TextureDimension::D2);
		let image = render::UIImage::new(state, &texture, position, size);
		Self { image, position, max: position + size }
	}

	pub fn render<'a>(&'a self, render_pass: &mut render::UIPass<'a>) {
		self.image.render(render_pass);
	}

	pub fn inside(&self, position: Vector<2, f32>) -> bool {
		self.position[X] <= position[X]
			&& position[X] < self.max[X]
			&& self.position[Y] <= position[Y]
			&& position[Y] < self.max[Y]
	}
}

pub enum UIAction {
	Nothing,
	Open,
	Debug,
}

impl Interface {
	pub fn new(state: &State) -> Self {
		Self {
			last_workload: 0,
			statistics: render::UIElement::new(
				vec![
					"...\n".into(),
					"...\n".into(),
					"...\n".into(),
					"...\n".into(),
				],
				[110.0, 10.0].into(),
				25.0,
			),
			show_statistics: false,
			open: Button::new(
				state,
				include_bytes!("../assets/folder-open.png"),
				[0.0, 0.0].into(),
				[100.0, 100.0].into(),
			),
			debug: Button::new(
				state,
				include_bytes!("../assets/debug.png"),
				[0.0, 100.0].into(),
				[100.0, 100.0].into(),
			),
		}
	}

	pub fn set_scale(&mut self, scale: f32) {
		self.statistics.position[X] = 110.0 * scale;
		self.statistics.font_size = 25.0 * scale;
	}

	pub fn update_fps(&mut self, fps: f64) {
		self.statistics.text[0] = format!("Max FPS: {:.0}\n", fps);
	}

	pub fn update_workload(&mut self, workload: usize) -> bool {
		if workload != self.last_workload {
			self.statistics.text[1] = format!("Chunks queued: {}\n", workload);
			self.last_workload = workload;
			true
		} else {
			false
		}
	}

	pub fn update_eye_dome_settings(&mut self, strength: f32, sensitivity: f32) {
		self.statistics.text[2] = format!("Highlight Strength: {}\n", strength);
		self.statistics.text[3] = format!("Highlight Sensitivity: {}\n", sensitivity);
	}

	pub fn clicked(&mut self, position: Vector<2, f32>) -> UIAction {
		if self.open.inside(position) {
			return UIAction::Open;
		}
		if self.debug.inside(position) {
			self.show_statistics = !self.show_statistics;
			return UIAction::Debug;
		}
		UIAction::Nothing
	}
}

impl render::UICollect for Interface {
	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		if self.show_statistics {
			collector.add_element(&self.statistics);
		}
	}
}

impl render::RenderableUI<State> for Interface {
	fn render<'a>(&'a self, mut render_pass: render::UIPass<'a>, _state: &'a State) -> render::UIPass<'a> {
		self.open.render(&mut render_pass);
		self.debug.render(&mut render_pass);
		render_pass
	}
}
