use math::{Vector, X, Y};

use crate::state::State;

pub struct Interface {
	pub last_workload: usize,
	pub statistics: render::UIElement,
	show_statistics: bool,
	close: Area,
	open: Area,
	debug: Area,
	color_palette: Area,
	property: Area,

	eye_dome: Area,
	eye_dome_expanded: bool,
	eye_dome_strength: Area,
	eye_dome_sensitivity: Area,

	camera: Area,

	level_of_detail: Area,
	level_of_detail_expanded: bool,
	level_of_detail_quality: Area,
}

struct Area {
	image: render::UIImage,
	position: Vector<2, f32>,
	max: Vector<2, f32>,
}

impl Area {
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

	pub fn inside_top(&self, position: Vector<2, f32>) -> bool {
		self.position[X] <= position[X]
			&& position[X] < self.max[X]
			&& self.position[Y] <= position[Y]
			&& position[Y] < (self.position[Y] + self.max[Y]) * 0.5
	}

	pub fn inside_bottom(&self, position: Vector<2, f32>) -> bool {
		self.position[X] <= position[X]
			&& position[X] < self.max[X]
			&& (self.position[Y] + self.max[Y]) * 0.5 <= position[Y]
			&& position[Y] < self.max[Y]
	}
}

#[derive(Clone, Copy, PartialEq)]
pub enum InterfaceAction {
	Nothing,
	Close,
	ReDraw,
	Open,
	ColorPalette,
	Property,
	Camera,
	LevelOfDetail,
	LevelOfDetailChange(f32),
	EyeDome,
	EyeDomeStrength(f32),
	EyeDomeSensitivity(f32),
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
			close: Area::new(
				state,
				include_bytes!("../assets/close-circle.png"),
				[0.0, 0.0].into(),
				[100.0, 100.0].into(),
			),
			open: Area::new(
				state,
				include_bytes!("../assets/folder-open.png"),
				[0.0, 100.0].into(),
				[100.0, 100.0].into(),
			),
			debug: Area::new(
				state,
				include_bytes!("../assets/debug.png"),
				[0.0, 200.0].into(),
				[100.0, 100.0].into(),
			),
			color_palette: Area::new(
				state,
				include_bytes!("../assets/color-palette.png"),
				[0.0, 300.0].into(),
				[100.0, 100.0].into(),
			),
			property: Area::new(
				state,
				include_bytes!("../assets/information-circle.png"),
				[0.0, 400.0].into(),
				[100.0, 100.0].into(),
			),
			eye_dome: Area::new(
				state,
				include_bytes!("../assets/invert-mode.png"),
				[0.0, 500.0].into(),
				[100.0, 100.0].into(),
			),
			eye_dome_expanded: false,
			eye_dome_sensitivity: Area::new(
				state,
				include_bytes!("../assets/chevron-expand.png"),
				[100.0, 500.0].into(),
				[100.0, 100.0].into(),
			),
			eye_dome_strength: Area::new(
				state,
				include_bytes!("../assets/chevron-expand.png"),
				[200.0, 500.0].into(),
				[100.0, 100.0].into(),
			),

			camera: Area::new(
				state,
				include_bytes!("../assets/videocam.png"),
				[0.0, 600.0].into(),
				[100.0, 100.0].into(),
			),
			level_of_detail: Area::new(
				state,
				include_bytes!("../assets/layers.png"),
				[0.0, 700.0].into(),
				[100.0, 100.0].into(),
			),
			level_of_detail_expanded: false,
			level_of_detail_quality: Area::new(
				state,
				include_bytes!("../assets/chevron-expand.png"),
				[100.0, 700.0].into(),
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

	pub fn clicked(&mut self, position: Vector<2, f32>) -> InterfaceAction {
		if self.close.inside(position) {
			return InterfaceAction::Close;
		}
		if self.open.inside(position) {
			return InterfaceAction::Open;
		}
		if self.debug.inside(position) {
			self.show_statistics = !self.show_statistics;
			return InterfaceAction::ReDraw;
		}
		if self.color_palette.inside(position) {
			return InterfaceAction::ColorPalette;
		}
		if self.property.inside(position) {
			return InterfaceAction::Property;
		}
		if self.eye_dome.inside(position) {
			return InterfaceAction::EyeDome;
		}
		if self.eye_dome_expanded {
			if self.eye_dome_strength.inside_top(position) {
				return InterfaceAction::EyeDomeStrength(5.0);
			}
			if self.eye_dome_strength.inside_bottom(position) {
				return InterfaceAction::EyeDomeStrength(-5.0);
			}

			if self.eye_dome_sensitivity.inside_top(position) {
				return InterfaceAction::EyeDomeSensitivity(5.0);
			}
			if self.eye_dome_sensitivity.inside_bottom(position) {
				return InterfaceAction::EyeDomeSensitivity(-5.0);
			}
		}

		if self.camera.inside(position) {
			return InterfaceAction::Camera;
		}
		if self.level_of_detail.inside(position) {
			return InterfaceAction::LevelOfDetail;
		}
		if self.level_of_detail_expanded {
			if self.level_of_detail_quality.inside_top(position) {
				return InterfaceAction::LevelOfDetailChange(5.0);
			}
			if self.level_of_detail_quality.inside_bottom(position) {
				return InterfaceAction::LevelOfDetailChange(-5.0);
			}
		}

		InterfaceAction::Nothing
	}

	pub fn scrolled(&mut self, position: Vector<2, f32>, delta: f32) -> InterfaceAction {
		if self.eye_dome_expanded {
			if self.eye_dome_strength.inside(position) {
				return InterfaceAction::EyeDomeStrength(delta);
			}
			if self.eye_dome_sensitivity.inside(position) {
				return InterfaceAction::EyeDomeSensitivity(delta);
			}
		}

		if self.level_of_detail_expanded && self.level_of_detail_quality.inside(position) {
			return InterfaceAction::LevelOfDetailChange(delta);
		}
		InterfaceAction::Nothing
	}

	pub fn hover(&mut self, position: Vector<2, f32>) -> InterfaceAction {
		let mut action = InterfaceAction::Nothing;
		if self.eye_dome_expanded {
			if !self.eye_dome.inside(position)
				&& !self.eye_dome_sensitivity.inside(position)
				&& !self.eye_dome_strength.inside(position)
			{
				self.eye_dome_expanded = false;
				action = InterfaceAction::ReDraw;
			}
		} else {
			if self.eye_dome.inside(position) {
				self.eye_dome_expanded = true;
				action = InterfaceAction::ReDraw;
			}
		}
		if self.level_of_detail_expanded {
			if !self.level_of_detail.inside(position) && !self.level_of_detail_quality.inside(position) {
				self.level_of_detail_expanded = false;
				action = InterfaceAction::ReDraw;
			}
		} else {
			if self.level_of_detail.inside(position) {
				self.level_of_detail_expanded = true;
				action = InterfaceAction::ReDraw;
			}
		}
		action
	}

	pub fn render<'a>(&'a self, mut render_pass: render::UIPass<'a>) -> render::UIPass<'a> {
		self.close.render(&mut render_pass);
		self.open.render(&mut render_pass);
		self.debug.render(&mut render_pass);
		self.color_palette.render(&mut render_pass);
		self.property.render(&mut render_pass);
		self.eye_dome.render(&mut render_pass);
		if self.eye_dome_expanded {
			self.eye_dome_strength.render(&mut render_pass);
			self.eye_dome_sensitivity.render(&mut render_pass);
		}
		self.camera.render(&mut render_pass);
		self.level_of_detail.render(&mut render_pass);
		if self.level_of_detail_expanded {
			self.level_of_detail_quality.render(&mut render_pass);
		}
		render_pass
	}
}

impl render::UICollect for Interface {
	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		if self.show_statistics {
			collector.add_element(&self.statistics);
		}
	}
}
