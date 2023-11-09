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
	SliceChange,
	SegmentReset,
}

ui::Collection!(
	type Event = InterfaceAction;

	pub struct Interface {
		left: ui::Area<Left>,
	}
);

type UpDownButton =
	ui::Split<ui::Horizontal, ui::Button<ui::Image<InterfaceAction>>, ui::Button<ui::Image<InterfaceAction>>>;

ui::List!(
	type Event = InterfaceAction;

	struct Left {
		open: ui::RelHeight<ui::Button<ui::Image<InterfaceAction>>>,
		color_palette: ui::RelHeight<ui::Button<ui::Image<InterfaceAction>>>,
		property: ui::RelHeight<ui::Button<ui::Image<InterfaceAction>>>,

		eye_dome: ui::Popup<ui::RelHeight<ui::Button<ui::Image<InterfaceAction>>>, ui::Area<UpDownButton>>,

		camera: ui::RelHeight<ui::Button<ui::Image<InterfaceAction>>>,
		level_of_detail: ui::Popup<ui::RelHeight<ui::Button<ui::Image<InterfaceAction>>>, ui::Area<UpDownButton>>,

		segment: ui::RelHeight<ui::Button<ui::Image<InterfaceAction>>>,
	}
);

impl Interface {
	pub fn new(state: &State) -> Self {
		Self {
			left: ui::Area::new(
				Left::new(state),
				ui::Anchor::new(
					[ui::Length::new(), ui::Length::new()].into(),
					[ui::Length::new().h(0.1), ui::Length::new().h(1.0)].into(),
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
						[ui::Length::new().w(1.0), ui::Length::new()].into(),
						ui::Length::new().h(1.0),
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
						[ui::Length::new().w(1.0), ui::Length::new()].into(),
						ui::Length::new().h(1.0),
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
		}
	}
}
