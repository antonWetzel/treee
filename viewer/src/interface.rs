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

	camera: Area,

	level_of_detail: Area,
	level_of_detail_expanded: bool,
	level_of_detail_quality: Area,

	slice: Area,
	slice_expanded: bool,
	slice_background: Area,
	slice_left: Area,
	slice_right: Area,
	pub slice_min: u32,
	pub slice_max: u32,

	segment: Area,
}

struct Area {
	image: render::UIImage,
	position: Vector<2, f32>,
	max: Vector<2, f32>,
	texture: render::Texture,
}

impl Area {
	pub fn new(state: &State, data: &[u8], position: Vector<2, f32>, size: Vector<2, f32>) -> Self {
		let texture = render::Texture::new(state, data, render::TextureDimension::D2);
		let image = render::UIImage::new(state, &texture, position, size);
		Self {
			image,
			position,
			max: position + size,
			texture,
		}
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

	pub fn change_position(&mut self, state: &State, position: Vector<2, f32>) {
		let size = self.max - self.position;
		self.position = position;
		self.max = position + size;
		self.image = render::UIImage::new(state, &self.texture, position, size);
	}
}

impl render::UIRender for Area {
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		ui_pass.render(&self.image);
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
	SliceChange,
	SegmentReset,
}

impl Interface {
	pub fn new(state: &State) -> Self {
		Self {
			last_workload: 0,
			statistics: render::UIElement::new(
				vec!["...\n".into(), "...\n".into(), "...\n".into()],
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
			eye_dome_strength: Area::new(
				state,
				include_bytes!("../assets/chevron-expand.png"),
				[100.0, 500.0].into(),
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

			slice: Area::new(
				state,
				include_bytes!("../assets/sliders.png"),
				[000.0, 800.0].into(),
				[100.0, 100.0].into(),
			),
			slice_expanded: false,
			slice_background: Area::new(
				state,
				include_bytes!("../assets/line.png"),
				[100.0, 800.0].into(),
				[300.0, 100.0].into(),
			),
			slice_left: Area::new(
				state,
				include_bytes!("../assets/dot.png"),
				[125.0, 825.0].into(),
				[50.0, 50.0].into(),
			),
			slice_right: Area::new(
				state,
				include_bytes!("../assets/dot.png"),
				[325.0, 825.0].into(),
				[50.0, 50.0].into(),
			),
			slice_min: u32::MIN,
			slice_max: u32::MAX,

			segment: Area::new(
				state,
				include_bytes!("../assets/cube.png"),
				[0.0, 900.0].into(),
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

	pub fn update_eye_dome_settings(&mut self, strength: f32) {
		self.statistics.text[2] = format!("Highlight Strength: {}\n", strength);
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
				return InterfaceAction::EyeDomeStrength(-5.0);
			}
			if self.eye_dome_strength.inside_bottom(position) {
				return InterfaceAction::EyeDomeStrength(5.0);
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

		if self.segment.inside(position) {
			return InterfaceAction::SegmentReset;
		}

		InterfaceAction::Nothing
	}

	pub fn scrolled(&mut self, position: Vector<2, f32>, delta: f32) -> InterfaceAction {
		if self.eye_dome_expanded && self.eye_dome_strength.inside(position) {
			return InterfaceAction::EyeDomeStrength(delta);
		}

		if self.level_of_detail_expanded && self.level_of_detail_quality.inside(position) {
			return InterfaceAction::LevelOfDetailChange(delta);
		}
		InterfaceAction::Nothing
	}

	pub fn hover(&mut self, position: Vector<2, f32>) -> InterfaceAction {
		let mut action = InterfaceAction::Nothing;
		if self.eye_dome_expanded && !self.eye_dome.inside(position) && !self.eye_dome_strength.inside(position) {
			self.eye_dome_expanded = false;
			action = InterfaceAction::ReDraw;
		}
		if !self.eye_dome_expanded && self.eye_dome.inside(position) {
			self.eye_dome_expanded = true;
			action = InterfaceAction::ReDraw;
		}

		if self.level_of_detail_expanded
			&& !self.level_of_detail.inside(position)
			&& !self.level_of_detail_quality.inside(position)
		{
			self.level_of_detail_expanded = false;
			action = InterfaceAction::ReDraw;
		}

		if !self.level_of_detail_expanded && self.level_of_detail.inside(position) {
			self.level_of_detail_expanded = true;
			action = InterfaceAction::ReDraw;
		}

		if self.slice_expanded && !self.slice.inside(position) && !self.slice_background.inside(position) {
			self.slice_expanded = false;
			action = InterfaceAction::ReDraw;
		}

		if !self.slice_expanded && self.slice.inside(position) {
			self.slice_expanded = true;
			action = InterfaceAction::ReDraw;
		}

		action
	}

	pub fn drag(&mut self, position: Vector<2, f32>, state: &State) -> InterfaceAction {
		let mut action = InterfaceAction::Nothing;
		if self.slice_expanded && self.slice_background.inside(position) {
			let value = (position[X] - self.slice_background.position[X] - 50.0) / 200.0;
			let value_percent = value.clamp(0.0, 1.0);
			let value = (value_percent * u32::MAX as f32) as u32;
			if value.abs_diff(self.slice_min) < value.abs_diff(self.slice_max) {
				self.slice_min = value;
				self.slice_left.change_position(
					state,
					self.slice_background.position + [25.0 + value_percent * 200.0, 25.0].into(),
				);
			} else {
				self.slice_max = value;
				self.slice_right.change_position(
					state,
					self.slice_background.position + [25.0 + value_percent * 200.0, 25.0].into(),
				);
			}

			action = InterfaceAction::SliceChange;
		}

		action
	}

	pub fn should_drag(&self, _position: Vector<2, f32>) -> bool {
		self.slice_expanded
	}
}

impl render::UICollect for Interface {
	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		if self.show_statistics {
			collector.add_element(&self.statistics);
		}
	}
}

impl render::UIRender for Interface {
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.close.render(ui_pass);
		self.open.render(ui_pass);
		self.debug.render(ui_pass);
		self.color_palette.render(ui_pass);
		self.property.render(ui_pass);
		self.eye_dome.render(ui_pass);
		if self.eye_dome_expanded {
			self.eye_dome_strength.render(ui_pass);
		}
		self.camera.render(ui_pass);
		self.level_of_detail.render(ui_pass);
		if self.level_of_detail_expanded {
			self.level_of_detail_quality.render(ui_pass);
		}

		self.slice.render(ui_pass);
		if self.slice_expanded {
			self.slice_background.render(ui_pass);
			self.slice_left.render(ui_pass);
			self.slice_right.render(ui_pass);
		}

		self.segment.render(ui_pass);
	}
}
