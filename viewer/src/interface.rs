use math::{Vector, X, Y, Z};

use crate::state::State;

#[derive(Clone, Copy, PartialEq)]
pub enum InterfaceAction {
	Open,

	BackgroundReset,
	BackgroundRed(f32),
	BackgroundGreen(f32),
	BackgroundBlue(f32),

	UpdateInterface,
	ColorPalette,
	Property,
	Camera,
	LevelOfDetail,
	LevelOfDetailChange(f32),

	EyeDome,
	EyeDomeStrength(f32),
	EyeDomeRed(f32),
	EyeDomeGreen(f32),
	EyeDomeBlue(f32),

	SliceUpdate(f32, f32),
	SegmentReset,

	ScaleUpdate(f32),
}

type Image = ui::Image<InterfaceAction>;
type Text = ui::Text<InterfaceAction>;

type UpDownButton = ui::Split<ui::Horizontal, ui::Button<Image>, ui::Button<Image>>;

ui::Collection!(
	type Event = InterfaceAction;

	pub struct Interface {
		controls: ui::Area<Controls>,

		info: ui::Hide<ui::Area<Text>>,
	}
);

impl Interface {
	pub fn new(state: &State, background: Vector<3, f32>) -> Self {
		Self {
			controls: ui::Area::new(
				Controls::new(state, background),
				ui::Anchor::new(
					[ui::Length::new(), ui::Length::new()].into(),
					[ui::length!(h 0.1), ui::length!(h 1.0)].into(),
				),
			),
			info: ui::Hide::new(
				ui::Area::new(
					Text::new(
						vec![String::from("hi")],
						ui::HorizontalAlign::Left,
						ui::VerticalAlign::Top,
					),
					ui::Anchor::new(
						[ui::length!(w 1.0, h -0.4), ui::length!(10.0)].into(),
						[ui::length!(h 0.2), ui::length!(-20.0, h 1.0)].into(),
					),
				),
				false,
			),
		}
	}

	pub fn reset_background(&mut self, state: &State, background: Vector<3, f32>) {
		self.controls
			.background
			.popup
			.red
			.second
			.set_marker(state, background[X]);
		self.controls
			.background
			.popup
			.green
			.second
			.set_marker(state, background[Y]);
		self.controls
			.background
			.popup
			.blue
			.second
			.set_marker(state, background[Z]);
	}

	pub fn enable_segment_info(&mut self, names: &[String], properties: &[common::Value]) {
		self.info.update_text(
			names
				.iter()
				.zip(properties)
				.map(|(name, prop)| format!("{}: {}\n", name, prop))
				.collect(),
		);
		self.info.active = true;
	}

	pub fn disable_segment_info(&mut self) {
		self.info.active = false;
	}
}

ui::Stack!(
	type Event = InterfaceAction;

	const DIRECTION = math::Y;

	struct Controls {
		open: ui::Relative<ui::Horizontal, ui::Button<Image>>,
		background: ui::Popup<ui::Relative<ui::Horizontal, ui::Button<Image>>, ui::Area<RGB>>,
		color_palette: ui::Relative<ui::Horizontal, ui::Button<Image>>,
		property: ui::Relative<ui::Horizontal, ui::Button<Image>>,

		eye_dome: ui::Popup<ui::Relative<ui::Horizontal,ui::Button<Image>>, ui::Area<EyeDome>>,

		camera: ui::Relative<ui::Horizontal,ui::Button<Image>>,
		level_of_detail: ui::Popup<ui::Relative<ui::Horizontal,ui::Button<Image>>, ui::Area<UpDownButton>>,

		segment: ui::Relative<ui::Horizontal,ui::Button<Image>>,

		slice: ui::Popup<ui::Relative<ui::Horizontal,ui::Button<Image>>, ui::Area<ui::DoubleSlider<ui::Horizontal, Image, Image>>>,
		scale: ui::Popup<ui::Relative<ui::Horizontal,ui::Button<Image>>, ui::Area<ui::Slider<ui::Horizontal, Image, Image>>>,
	}
);

ui::Stack!(
	type Event = InterfaceAction;

	const DIRECTION = math::Y;

	struct RGB {
		red: ui::Relative<ui::Horizontal, ui::Split<ui::Vertical, Text, ui::Slider<ui::Horizontal, Image, Image>>>,
		green: ui::Relative<ui::Horizontal, ui::Split<ui::Vertical, Text, ui::Slider<ui::Horizontal, Image, Image>>>,
		blue: ui::Relative<ui::Horizontal, ui::Split<ui::Vertical, Text, ui::Slider<ui::Horizontal, Image, Image>>>,
	}
);

impl RGB {
	pub fn new(
		state: &State,
		line: &render::Texture,
		dot: &render::Texture,
		r: fn(f32) -> InterfaceAction,
		g: fn(f32) -> InterfaceAction,
		b: fn(f32) -> InterfaceAction,
		default: Vector<3, f32>,
	) -> Self {
		Self {
			red: ui::Relative::new(
				ui::Split::new(
					ui::Text::new(
						vec![String::from("R:")],
						ui::HorizontalAlign::Right,
						ui::VerticalAlign::Center,
					),
					ui::Slider::new(
						ui::Image::new(state, line),
						ui::Image::new(state, dot),
						default[X],
						r,
					),
					0.25,
				),
				1.0 / 4.0,
			),
			green: ui::Relative::new(
				ui::Split::new(
					ui::Text::new(
						vec![String::from("G:")],
						ui::HorizontalAlign::Right,
						ui::VerticalAlign::Center,
					),
					ui::Slider::new(
						ui::Image::new(state, line),
						ui::Image::new(state, dot),
						default[Y],
						g,
					),
					0.25,
				),
				1.0 / 4.0,
			),
			blue: ui::Relative::new(
				ui::Split::new(
					ui::Text::new(
						vec![String::from("B:")],
						ui::HorizontalAlign::Right,
						ui::VerticalAlign::Center,
					),
					ui::Slider::new(
						ui::Image::new(state, line),
						ui::Image::new(state, dot),
						default[Z],
						b,
					),
					0.25,
				),
				1.0 / 4.0,
			),
		}
	}
}

ui::Stack!(
	type Event = InterfaceAction;

	const DIRECTION = math::X;

	struct EyeDome {
		strength: ui::Relative<ui::Vertical, ui::Slider<ui::Vertical, Image, Image>>,
		rgb: RGB,
	}
);

impl EyeDome {
	pub fn new(state: &State, line_h: &render::Texture, line_v: &render::Texture, dot: &render::Texture) -> Self {
		Self {
			strength: ui::Relative::new(
				ui::Slider::new(
					ui::Image::new(state, line_v),
					ui::Image::new(state, dot),
					0.3,
					InterfaceAction::EyeDomeStrength,
				),
				1.0 / 3.0,
			),
			rgb: RGB::new(
				state,
				line_h,
				dot,
				InterfaceAction::EyeDomeRed,
				InterfaceAction::EyeDomeGreen,
				InterfaceAction::EyeDomeBlue,
				[0.0, 0.0, 0.0].into(),
			),
		}
	}
}

impl Controls {
	pub fn new(state: &State, background: Vector<3, f32>) -> Self {
		let up = render::Texture::new(
			state,
			include_bytes!("../assets/png/chevron-expand-top.png"),
		);
		let down = render::Texture::new(
			state,
			include_bytes!("../assets/png/chevron-expand-bottom.png"),
		);
		let dot = render::Texture::new(state, include_bytes!("../assets/png/dot.png"));
		let line_h = render::Texture::new(state, include_bytes!("../assets/png/line-h.png"));
		let line_v = render::Texture::new(state, include_bytes!("../assets/png/line-v.png"));

		Self {
			open: ui::Relative::square(ui::Button::new(
				ui::Image::new(
					state,
					&render::Texture::new(state, include_bytes!("../assets/png/folder-open.png")),
				),
				|| InterfaceAction::Open,
			)),
			background: ui::Popup::new(
				ui::Relative::square(ui::Button::new(
					ui::Image::new(
						state,
						&render::Texture::new(state, include_bytes!("../assets/png/paint-bucket.png")),
					),
					|| InterfaceAction::BackgroundReset,
				)),
				ui::Area::new(
					RGB::new(
						state,
						&line_h,
						&dot,
						InterfaceAction::BackgroundRed,
						InterfaceAction::BackgroundGreen,
						InterfaceAction::BackgroundBlue,
						background,
					),
					ui::Anchor::new(
						[ui::length!(w 1.0), ui::length!()].into(),
						[ui::length!(h 4.0), ui::length!(h 3.0)].into(),
					),
				),
				|| InterfaceAction::UpdateInterface,
			),
			color_palette: ui::Relative::square(ui::Button::new(
				ui::Image::new(
					state,
					&render::Texture::new(state, include_bytes!("../assets/png/color-palette.png")),
				),
				|| InterfaceAction::ColorPalette,
			)),
			property: ui::Relative::square(ui::Button::new(
				ui::Image::new(
					state,
					&render::Texture::new(
						state,
						include_bytes!("../assets/png/information-circle.png"),
					),
				),
				|| InterfaceAction::Property,
			)),
			eye_dome: ui::Popup::new(
				ui::Relative::square(ui::Button::new(
					ui::Image::new(
						state,
						&render::Texture::new(state, include_bytes!("../assets/png/invert-mode.png")),
					),
					|| InterfaceAction::EyeDome,
				)),
				ui::Area::new(
					EyeDome::new(state, &line_h, &line_v, &dot),
					ui::Anchor::new(
						[ui::length!(w 1.0), ui::length!()].into(),
						[ui::length!(h 5.0), ui::length!(h 3.0)].into(),
					),
				),
				|| InterfaceAction::UpdateInterface,
			),

			camera: ui::Relative::square(ui::Button::new(
				ui::Image::new(
					state,
					&render::Texture::new(state, include_bytes!("../assets/png/videocam.png")),
				),
				|| InterfaceAction::Camera,
			)),

			level_of_detail: ui::Popup::new(
				ui::Relative::square(ui::Button::new(
					ui::Image::new(
						state,
						&render::Texture::new(state, include_bytes!("../assets/png/layers.png")),
					),
					|| InterfaceAction::LevelOfDetail,
				)),
				ui::Area::new(
					ui::Split::new(
						ui::Button::new(ui::Image::new(state, &up), || {
							InterfaceAction::LevelOfDetailChange(-5.0)
						}),
						ui::Button::new(ui::Image::new(state, &down), || {
							InterfaceAction::LevelOfDetailChange(5.0)
						}),
						0.5,
					),
					ui::Anchor::square(
						[ui::length!(w 1.0), ui::length!()].into(),
						ui::length!(h 1.0),
					),
				),
				|| InterfaceAction::UpdateInterface,
			),

			segment: ui::Relative::square(ui::Button::new(
				ui::Image::new(
					state,
					&render::Texture::new(state, include_bytes!("../assets/png/cube.png")),
				),
				|| InterfaceAction::SegmentReset,
			)),

			slice: ui::Popup::new(
				ui::Relative::square(ui::Button::new(
					ui::Image::new(
						state,
						&render::Texture::new(state, include_bytes!("../assets/png/sliders.png")),
					),
					|| InterfaceAction::UpdateInterface,
				)),
				ui::Area::new(
					ui::DoubleSlider::new(
						ui::Image::new(state, &line_h),
						ui::Image::new(state, &dot),
						ui::Image::new(state, &dot),
						InterfaceAction::SliceUpdate,
					),
					ui::Anchor::new(
						[ui::length!(w 1.0), ui::length!()].into(),
						[ui::length!(w 3.0), ui::length!(h 1.0)].into(),
					),
				),
				|| InterfaceAction::UpdateInterface,
			),

			scale: ui::Popup::new(
				ui::Relative::square(ui::Button::new(
					ui::Image::new(
						state,
						&render::Texture::new(state, include_bytes!("../assets/png/sliders.png")),
					),
					|| InterfaceAction::UpdateInterface,
				)),
				ui::Area::new(
					ui::Slider::new(
						ui::Image::new(state, &line_h),
						ui::Image::new(state, &dot),
						0.5,
						InterfaceAction::ScaleUpdate,
					),
					ui::Anchor::new(
						[ui::length!(w 1.0), ui::length!()].into(),
						[ui::length!(w 3.0), ui::length!(h 1.0)].into(),
					),
				),
				|| InterfaceAction::UpdateInterface,
			),
		}
	}
}
