use math::Vector;

use crate::state::State;

#[derive(Clone, Copy, PartialEq)]
pub enum InterfaceAction {
	Close,
	Open,
	UpdateInterface,
	ColorPalette,
	Property,
	Camera,
	Slice,
	LevelOfDetail,
	LevelOfDetailChange(f32),
	EyeDome,
	EyeDomeStrength(f32),
	SliceChange,
	SegmentReset,
}

ui::UICollection!(
	type Event = InterfaceAction;

	pub struct Interface {
		close: ui::Area<ui::Button<ui::Image<InterfaceAction>>>,
		open: ui::Area<ui::Button<ui::Image<InterfaceAction>>>,
		color_palette: ui::Area<ui::Button<ui::Image<InterfaceAction>>>,
		property: ui::Area<ui::Button<ui::Image<InterfaceAction>>>,
	}
);

impl Interface {
	pub fn new(state: &State) -> Self {
		Self {
			close: ui::Area::new(
				ui::Button::new(
					ui::Image::new(
						state,
						&render::Texture::new(state, include_bytes!("../assets/close-circle.png")),
					),
					|| InterfaceAction::Close,
				),
				ui::Anchor::square(
					[ui::Size::new(), ui::Size::new()].into(),
					ui::Size::new().height(0.1),
				),
			),
			open: ui::Area::new(
				ui::Button::new(
					ui::Image::new(
						state,
						&render::Texture::new(state, include_bytes!("../assets/folder-open.png")),
					),
					|| InterfaceAction::Open,
				),
				ui::Anchor::square(
					[ui::Size::new(), ui::Size::new().height(0.1)].into(),
					ui::Size::new().height(0.1),
				),
			),
			color_palette: ui::Area::new(
				ui::Button::new(
					ui::Image::new(
						state,
						&render::Texture::new(state, include_bytes!("../assets/color-palette.png")),
					),
					|| InterfaceAction::ColorPalette,
				),
				ui::Anchor::square(
					[ui::Size::new(), ui::Size::new().height(0.2)].into(),
					ui::Size::new().height(0.1),
				),
			),
			property: ui::Area::new(
				ui::Button::new(
					ui::Image::new(
						state,
						&render::Texture::new(state, include_bytes!("../assets/information-circle.png")),
					),
					|| InterfaceAction::Property,
				),
				ui::Anchor::square(
					[ui::Size::new(), ui::Size::new().height(0.3)].into(),
					ui::Size::new().height(0.1),
				),
			),
		}
	}
}

// pub struct Interface {
// 	last_workload: usize,
// 	statistics: ui::UIText,
// 	show_statistics: bool,
// 	close: Area,
// 	open: Area,
// 	debug: Area,
// 	color_palette: Area,
// 	property: Area,

// 	eye_dome: Area,
// 	// eye_dome_expanded: bool,
// 	// eye_dome_strength: Area,
// 	camera: Area,

// 	level_of_detail: Area,
// 	// level_of_detail_expanded: bool,
// 	// level_of_detail_quality: Area,
// 	slice: Area,
// 	// slice_expanded: bool,
// 	// slice_background: Area,
// 	// slice_left: Area,
// 	// slice_right: Area,
// 	slice_min: u32,
// 	slice_max: u32,

// 	segment: Area,

// 	segment_info_active: bool,
// 	segment_info: ui::UIText,

// 	test: ui::UIHoverPopup<
// 		ui::UIArea<ui::UIImage<InterfaceAction>>,
// 		ui::UIArea<ui::UIButton<ui::UIImage<InterfaceAction>>>,
// 	>,
// }

// struct Area {
// 	image: ui::UIImage,
// 	position: Vector<2, f32>,
// 	max: Vector<2, f32>,
// 	texture: ui::Texture,
// }

// impl Area {
// 	pub fn new(state: &State, data: &[u8], position: Vector<2, f32>, size: Vector<2, f32>) -> Self {
// 		let texture = ui::Texture::new(state, data, ui::TextureDimension::D2);
// 		let image = ui::UIImage::new(state, &texture, position, size);
// 		Self {
// 			image,
// 			position,
// 			max: position + size,
// 			texture,
// 		}
// 	}

// 	pub fn inside(&self, position: Vector<2, f32>) -> bool {
// 		self.position[X] <= position[X]
// 			&& position[X] < self.max[X]
// 			&& self.position[Y] <= position[Y]
// 			&& position[Y] < self.max[Y]
// 	}

// 	pub fn inside_top(&self, position: Vector<2, f32>) -> bool {
// 		self.position[X] <= position[X]
// 			&& position[X] < self.max[X]
// 			&& self.position[Y] <= position[Y]
// 			&& position[Y] < (self.position[Y] + self.max[Y]) * 0.5
// 	}

// 	pub fn inside_bottom(&self, position: Vector<2, f32>) -> bool {
// 		self.position[X] <= position[X]
// 			&& position[X] < self.max[X]
// 			&& (self.position[Y] + self.max[Y]) * 0.5 <= position[Y]
// 			&& position[Y] < self.max[Y]
// 	}

// 	pub fn change_position(&mut self, state: &State, position: Vector<2, f32>) {
// 		let size = self.max - self.position;
// 		self.position = position;
// 		self.max = position + size;
// 		self.image = ui::UIImage::new(state, &self.texture, position, size);
// 	}
// }

// impl ui::UIRender for Area {
// 	fn ui<'a>(&'a self, ui_pass: &mut ui::UIPass<'a>) {
// 		ui_pass.ui(&self.image);
// 	}
// }

// impl Interface {
// 	pub fn new(state: &State) -> Self {
// 		fn area(state: &State, data: &[u8], position: Vector<2, f32>, action: InterfaceAction) -> Area {
// 			let size = ui::UISize {
// 				absolute: 0.0,
// 				relative_height: 0.1,
// 				relative_width: 0.0,
// 			};
// 			let position = [
// 				ui::UISize {
// 					absolute: 30.0,
// 					relative_height: 0.1 * position[X],
// 					relative_width: 0.0,
// 				},
// 				ui::UISize {
// 					absolute: 0.0,
// 					relative_height: 0.1 * position[Y],
// 					relative_width: 0.0,
// 				},
// 			]
// 			.into();
// 			ui::UIArea::new(
// 				ui::UIButton::new(
// 					ui::UIImage::new(
// 						state,
// 						&ui::Texture::new(state, data, ui::TextureDimension::D2),
// 					),
// 					action,
// 				),
// 				ui::UIAnchor { position, size: [size, size].into() },
// 			)
// 		}

// 		Self {
// 			last_workload: 0,
// 			statistics: ui::UIText::new(
// 				vec!["...\n".into(), "...\n".into(), "...\n".into()],
// 				[110.0, 10.0].into(),
// 				25.0,
// 			),
// 			show_statistics: false,
// 			close: area(
// 				state,
// 				include_bytes!("../assets/close-circle.png"),
// 				[0.0, 0.0].into(),
// 				InterfaceAction::Close,
// 			),
// 			open: area(
// 				state,
// 				include_bytes!("../assets/folder-open.png"),
// 				[0.0, 1.0].into(),
// 				InterfaceAction::Open,
// 			),
// 			debug: area(
// 				state,
// 				include_bytes!("../assets/debug.png"),
// 				[0.0, 2.0].into(),
// 				InterfaceAction::Redraw,
// 			),
// 			color_palette: area(
// 				state,
// 				include_bytes!("../assets/color-palette.png"),
// 				[0.0, 3.0].into(),
// 				InterfaceAction::ColorPalette,
// 			),
// 			property: area(
// 				state,
// 				include_bytes!("../assets/information-circle.png"),
// 				[0.0, 4.0].into(),
// 				InterfaceAction::Property,
// 			),
// 			eye_dome: area(
// 				state,
// 				include_bytes!("../assets/invert-mode.png"),
// 				[0.0, 5.0].into(),
// 				InterfaceAction::EyeDome,
// 			),
// 			// eye_dome_expanded: false,
// 			// eye_dome_strength: area(
// 			// 	state,
// 			// 	include_bytes!("../assets/chevron-expand.png"),
// 			// 	[100.0, 500.0].into(),
// 			// 	[100.0, 100.0].into(),
// 			// ),
// 			camera: area(
// 				state,
// 				include_bytes!("../assets/videocam.png"),
// 				[0.0, 6.0].into(),
// 				InterfaceAction::Camera,
// 			),
// 			level_of_detail: area(
// 				state,
// 				include_bytes!("../assets/layers.png"),
// 				[0.0, 7.0].into(),
// 				InterfaceAction::LevelOfDetail,
// 			),
// 			// level_of_detail_expanded: false,
// 			// level_of_detail_quality: area(
// 			// 	state,
// 			// 	include_bytes!("../assets/chevron-expand.png"),
// 			// 	[100.0, 700.0].into(),
// 			// 	[100.0, 100.0].into(),
// 			// ),
// 			slice: area(
// 				state,
// 				include_bytes!("../assets/sliders.png"),
// 				[000.0, 8.0].into(),
// 				InterfaceAction::Slice,
// 			),
// 			// slice_expanded: false,
// 			// slice_background: area(
// 			// 	state,
// 			// 	include_bytes!("../assets/line.png"),
// 			// 	[100.0, 800.0].into(),
// 			// 	[300.0, 100.0].into(),
// 			// ),
// 			// slice_left: area(
// 			// 	state,
// 			// 	include_bytes!("../assets/dot.png"),
// 			// 	[125.0, 825.0].into(),
// 			// 	[50.0, 50.0].into(),
// 			// ),
// 			// slice_right: area(
// 			// 	state,
// 			// 	include_bytes!("../assets/dot.png"),
// 			// 	[325.0, 825.0].into(),
// 			// 	[50.0, 50.0].into(),
// 			// ),
// 			slice_min: u32::MIN,
// 			slice_max: u32::MAX,

// 			segment: area(
// 				state,
// 				include_bytes!("../assets/cube.png"),
// 				[0.0, 9.0].into(),
// 				InterfaceAction::SegmentReset,
// 			),

// 			segment_info: ui::UIText::new(Vec::new(), Vector::default(), 25.0),
// 			segment_info_active: false,

// 			test: ui::UIHoverPopup::new(
// 				ui::UIArea::new(
// 					ui::UIImage::new(
// 						state,
// 						&ui::Texture::new(
// 							state,
// 							include_bytes!("../assets/cube.png"),
// 							ui::TextureDimension::D2,
// 						),
// 					),
// 					ui::UIAnchor {
// 						position: [
// 							ui::UISize {
// 								absolute: 0.0,
// 								relative_height: 0.3,
// 								relative_width: 0.0,
// 							},
// 							ui::UISize {
// 								absolute: 0.0,
// 								relative_height: 0.3,
// 								relative_width: 0.0,
// 							},
// 						]
// 						.into(),
// 						size: [
// 							ui::UISize {
// 								absolute: 0.0,
// 								relative_height: 0.1,
// 								relative_width: 0.0,
// 							},
// 							ui::UISize {
// 								absolute: 0.0,
// 								relative_height: 0.1,
// 								relative_width: 0.0,
// 							},
// 						]
// 						.into(),
// 					},
// 				),
// 				ui::UIArea::new(
// 					ui::UIButton::new(
// 						ui::UIImage::new(
// 							state,
// 							&ui::Texture::new(
// 								state,
// 								include_bytes!("../assets/cube.png"),
// 								ui::TextureDimension::D2,
// 							),
// 						),
// 						InterfaceAction::ColorPalette,
// 					),
// 					ui::UIAnchor {
// 						position: [
// 							ui::UISize {
// 								absolute: 0.0,
// 								relative_height: 0.4,
// 								relative_width: 0.0,
// 							},
// 							ui::UISize {
// 								absolute: 0.0,
// 								relative_height: 0.3,
// 								relative_width: 0.0,
// 							},
// 						]
// 						.into(),
// 						size: [
// 							ui::UISize {
// 								absolute: 0.0,
// 								relative_height: 0.2,
// 								relative_width: 0.0,
// 							},
// 							ui::UISize {
// 								absolute: 0.0,
// 								relative_height: 0.1,
// 								relative_width: 0.0,
// 							},
// 						]
// 						.into(),
// 					},
// 				),
// 				InterfaceAction::Redraw,
// 			),
// 		}
// 	}

// pub fn update_fps(&mut self, fps: f64) {
// 	self.statistics.text[0] = format!("Max FPS: {:.0}\n", fps);
// }

// pub fn update_workload(&mut self, workload: usize) -> bool {
// 	if workload != self.last_workload {
// 		self.statistics.text[1] = format!("Chunks queued: {}\n", workload);
// 		self.last_workload = workload;
// 		true
// 	} else {
// 		false
// 	}
// }

// pub fn update_eye_dome_settings(&mut self, strength: f32) {
// 	self.statistics.text[2] = format!("Highlight Strength: {}\n", strength);
// }

// // pub fn clicked(&mut self, position: Vector<2, f32>) -> Option<InterfaceAction> {
// // if self.close.inside(position) {
// // 	return InterfaceAction::Close;
// // }
// // if self.open.inside(position) {
// // 	return InterfaceAction::Open;
// // }
// // if self.debug.inside(position) {
// // 	self.show_statistics = !self.show_statistics;
// // 	return InterfaceAction::ReDraw;
// // }
// // if self.color_palette.inside(position) {
// // 	return InterfaceAction::ColorPalette;
// // }
// // if self.property.inside(position) {
// // 	return InterfaceAction::Property;
// // }
// // if self.eye_dome.inside(position) {
// // 	return InterfaceAction::EyeDome;
// // }
// // if self.eye_dome_expanded {
// // 	if self.eye_dome_strength.inside_top(position) {
// // 		return InterfaceAction::EyeDomeStrength(-5.0);
// // 	}
// // 	if self.eye_dome_strength.inside_bottom(position) {
// // 		return InterfaceAction::EyeDomeStrength(5.0);
// // 	}
// // }

// // if self.camera.inside(position) {
// // 	return InterfaceAction::Camera;
// // }
// // if self.level_of_detail.inside(position) {
// // 	return InterfaceAction::LevelOfDetail;
// // }
// // if self.level_of_detail_expanded {
// // 	if self.level_of_detail_quality.inside_top(position) {
// // 		return InterfaceAction::LevelOfDetailChange(5.0);
// // 	}
// // 	if self.level_of_detail_quality.inside_bottom(position) {
// // 		return InterfaceAction::LevelOfDetailChange(-5.0);
// // 	}
// // }

// // if self.segment.inside(position) {
// // 	return InterfaceAction::SegmentReset;
// // }

// // InterfaceAction::Nothing

// // 	None
// // }

// pub fn drag(&mut self, position: Vector<2, f32>, state: &State) -> Option<InterfaceAction> {
// 	// let mut action = InterfaceAction::Nothing;
// 	// if self.slice_expanded && self.slice_background.inside(position) {
// 	// 	let value = (position[X] - self.slice_background.position[X] - 50.0) / 200.0;
// 	// 	let value_percent = value.clamp(0.0, 1.0);
// 	// 	let value = (value_percent * u32::MAX as f32) as u32;
// 	// 	if value.abs_diff(self.slice_min) < value.abs_diff(self.slice_max) {
// 	// 		self.slice_min = value;
// 	// 		self.slice_left.change_position(
// 	// 			state,
// 	// 			self.slice_background.position + [25.0 + value_percent * 200.0, 25.0].into(),
// 	// 		);
// 	// 	} else {
// 	// 		self.slice_max = value;
// 	// 		self.slice_right.change_position(
// 	// 			state,
// 	// 			self.slice_background.position + [25.0 + value_percent * 200.0, 25.0].into(),
// 	// 		);
// 	// 	}

// 	// 	action = InterfaceAction::SliceChange;
// 	// }

// 	// action

// 	None
// }

// pub fn slice_bounds(&self) -> (u32, u32) {
// 	(self.slice_min, self.slice_max)
// }

// pub fn enable_segment_info(&mut self, names: &[String], values: &[common::Value]) {
// 	self.segment_info_active = true;
// 	self.segment_info.text = names
// 		.iter()
// 		.zip(values)
// 		.map(|(name, value)| format!("{}: {}\n", name, value))
// 		.collect();
// }

// pub fn disable_segment_info(&mut self) {
// 	self.segment_info_active = false;
// }
// }

// impl ui::UIElement for Interface {
// 	type Event = InterfaceAction;

// 	fn inside(&self, _position: Vector<2, f32>) -> bool {
// 		true
// 	}

// 	fn resize(
// 		&mut self,
// 		state: &(impl ui::Has<ui::State> + ui::Has<ui::UIState>),
// 		rect: ui::UIRect,
// 	) {
// 		// self.statistics.position[X] = 110.0 * scale;
// 		// self.statistics.font_size = 25.0 * scale;
// 		// self.segment_info.font_size = 25.0 * scale;
// 		// self.segment_info.position[X] = size[X] - 400.0 * scale;

// 		self.close.resize(state, rect);
// 		self.open.resize(state, rect);
// 		self.test.resize(state, rect);
// 	}

// 	fn click(&mut self, position: Vector<2, f32>) -> Option<Self::Event> {
// 		self.close
// 			.click(position)
// 			.or_else(|| self.open.click(position))
// 			.or_else(|| self.test.click(position))
// 	}

// 	fn collect<'a>(&'a self, collector: &mut ui::UICollector<'a>) {
// 		if self.show_statistics {
// 			collector.add_element(&self.statistics);
// 		}
// 		if self.segment_info_active {
// 			collector.add_element(&self.segment_info);
// 		}
// 	}

// 	fn ui<'a>(&'a self, ui_pass: &mut ui::UIPass<'a>) {
// 		self.close.ui(ui_pass);
// 		self.open.ui(ui_pass);
// 		self.debug.ui(ui_pass);
// 		self.color_palette.ui(ui_pass);
// 		self.property.ui(ui_pass);
// 		self.eye_dome.ui(ui_pass);
// 		// if self.eye_dome_expanded {
// 		// 	self.eye_dome_strength.ui(ui_pass);
// 		// }
// 		self.camera.ui(ui_pass);
// 		self.level_of_detail.ui(ui_pass);
// 		// if self.level_of_detail_expanded {
// 		// 	self.level_of_detail_quality.ui(ui_pass);
// 		// }

// 		self.slice.ui(ui_pass);
// 		// if self.slice_expanded {
// 		// 	self.slice_background.ui(ui_pass);
// 		// 	self.slice_left.ui(ui_pass);
// 		// 	self.slice_right.ui(ui_pass);
// 		// }

// 		self.segment.ui(ui_pass);

// 		self.test.ui(ui_pass);
// 	}

// 	fn hover(&mut self, position: Vector<2, f32>) -> Option<Self::Event> {
// 		self.test.hover(position)
// 	}
// }
