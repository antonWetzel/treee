use math::Vector;

use crate::state::State;

#[derive(Clone, Copy, PartialEq)]
pub enum InterfaceAction {
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
	SliceUpdate(f32, f32),
	SegmentReset,
}

ui::Collection!(
	type Event = InterfaceAction;

	pub struct Interface {
		left: ui::Area<Left>,
	}
);

type Image = ui::Image<InterfaceAction>;

type UpDownButton = ui::Split<ui::Horizontal, ui::Button<Image>, ui::Button<Image>>;

ui::List!(
	type Event = InterfaceAction;

	struct Left {
		open: ui::RelHeight<ui::Button<Image>>,
		color_palette: ui::RelHeight<ui::Button<Image>>,
		property: ui::RelHeight<ui::Button<Image>>,

		eye_dome: ui::Popup<ui::RelHeight<ui::Button<Image>>, ui::Area<UpDownButton>>,

		camera: ui::RelHeight<ui::Button<Image>>,
		level_of_detail: ui::Popup<ui::RelHeight<ui::Button<Image>>, ui::Area<UpDownButton>>,

		segment: ui::RelHeight<ui::Button<Image>>,

		slice: ui::Popup<ui::RelHeight<ui::Button<Image>>, ui::Area<ui::DoubleSlider<Image, Image>>>,
	}
);

impl Interface {
	pub fn new(state: &State) -> Self {
		Self {
			left: ui::Area::new(
				Left::new(state),
				ui::Anchor::new(
					[ui::Length::new(), ui::Length::new()].into(),
					[ui::length!(h 0.1), ui::length!(h 1.0)].into(),
				),
			),
		}
	}
}

impl Left {
	pub fn new(state: &State) -> Self {
		let up = render::Texture::new(state, include_bytes!("../assets/chevron-expand-top.png"));
		let down = render::Texture::new(state, include_bytes!("../assets/chevron-expand-bottom.png"));
		Self {
			open: ui::RelHeight::square(ui::Button::new(
				ui::Image::new(
					state,
					&render::Texture::new(state, include_bytes!("../assets/folder-open.png")),
				),
				|| InterfaceAction::Open,
			)),
			color_palette: ui::RelHeight::square(ui::Button::new(
				ui::Image::new(
					state,
					&render::Texture::new(state, include_bytes!("../assets/color-palette.png")),
				),
				|| InterfaceAction::ColorPalette,
			)),
			property: ui::RelHeight::square(ui::Button::new(
				ui::Image::new(
					state,
					&render::Texture::new(state, include_bytes!("../assets/information-circle.png")),
				),
				|| InterfaceAction::Property,
			)),
			eye_dome: ui::Popup::new(
				ui::RelHeight::square(ui::Button::new(
					ui::Image::new(
						state,
						&render::Texture::new(state, include_bytes!("../assets/invert-mode.png")),
					),
					|| InterfaceAction::EyeDome,
				)),
				ui::Area::new(
					ui::Split::new(
						ui::Button::new(ui::Image::new(state, &up), || {
							InterfaceAction::EyeDomeStrength(-5.0)
						}),
						ui::Button::new(ui::Image::new(state, &down), || {
							InterfaceAction::EyeDomeStrength(5.0)
						}),
					),
					ui::Anchor::square(
						[ui::length!(w 1.0), ui::length!()].into(),
						ui::length!(h 1.0),
					),
				),
				|| InterfaceAction::UpdateInterface,
			),

			camera: ui::RelHeight::square(ui::Button::new(
				ui::Image::new(
					state,
					&render::Texture::new(state, include_bytes!("../assets/videocam.png")),
				),
				|| InterfaceAction::Camera,
			)),

			level_of_detail: ui::Popup::new(
				ui::RelHeight::square(ui::Button::new(
					ui::Image::new(
						state,
						&render::Texture::new(state, include_bytes!("../assets/layers.png")),
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
					),
					ui::Anchor::square(
						[ui::length!(w 1.0), ui::length!()].into(),
						ui::length!(h 1.0),
					),
				),
				|| InterfaceAction::UpdateInterface,
			),

			segment: ui::RelHeight::square(ui::Button::new(
				ui::Image::new(
					state,
					&render::Texture::new(state, include_bytes!("../assets/cube.png")),
				),
				|| InterfaceAction::SegmentReset,
			)),

			slice: ui::Popup::new(
				ui::RelHeight::square(ui::Button::new(
					ui::Image::new(
						state,
						&render::Texture::new(state, include_bytes!("../assets/sliders.png")),
					),
					|| InterfaceAction::UpdateInterface,
				)),
				ui::Area::new(
					ui::DoubleSlider::new(
						ui::Image::new(
							state,
							&render::Texture::new(state, include_bytes!("../assets/line.png")),
						),
						ui::Image::new(
							state,
							&render::Texture::new(state, include_bytes!("../assets/dot.png")),
						),
						ui::Image::new(
							state,
							&render::Texture::new(state, include_bytes!("../assets/dot.png")),
						),
						|lower, upper| InterfaceAction::SliceUpdate(lower, upper),
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
