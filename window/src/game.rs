use std::sync::Arc;

use math::{Vector, X, Y};
use render::Window;

use crate::{tree::TreeContext, State};

pub trait CustomState {
	type Tree: AsRef<TreeContext> + AsMut<TreeContext>;
}

pub struct Game<TCustomState: CustomState> {
	pub tree: TCustomState::Tree,
	pub custom_state: TCustomState,

	pub state: Arc<State>, // todo: this was the only public, make the rest private again
	pub mouse: input::Mouse,
	pub mouse_start: Option<Vector<2, f32>>,

	pub keyboard: input::Keyboard,
	pub time: Time,
	pub paused: bool,

	pub property_options: bool,
	pub visual_options: bool,
	pub level_of_detail_options: bool,
	pub camera_options: bool,
	pub quit: bool,
}

impl<TCustomState: CustomState> Game<TCustomState> {
	pub fn new(tree: TCustomState::Tree, state: Arc<State>, custom_state: TCustomState) -> Self {
		Self {
			tree,
			custom_state,
			paused: false,
			state,
			mouse: input::Mouse::new(),
			mouse_start: None,
			keyboard: input::Keyboard::new(),
			time: Time::new(),

			property_options: false,
			level_of_detail_options: false,
			camera_options: false,
			visual_options: false,
			quit: false,
		}
	}
	pub fn resize_window(&mut self, window: &mut Window, size: Vector<2, u32>) {
		self.paused = size[X] == 0 || size[Y] == 0;
		if self.paused {
			return;
		}
		window.resized(&self.state);
		self.tree
			.as_mut()
			.camera
			.cam
			.set_aspect(window.get_aspect());
		self.tree
			.as_mut()
			.eye_dome
			.update_depth(&self.state, &window.depth_texture());
		self.tree.as_mut().camera.gpu = render::Camera3DGPU::new(
			&self.state,
			&self.tree.as_ref().camera.cam,
			&self.tree.as_ref().camera.transform,
		);
	}
}

impl<T: CustomState> std::ops::Deref for Game<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.custom_state
	}
}

pub struct Time {
	last: std::time::Instant,
}

impl Time {
	pub fn new() -> Self {
		Self { last: std::time::Instant::now() }
	}

	pub fn elapsed(&mut self) -> std::time::Duration {
		let delta = self.last.elapsed();
		self.last = std::time::Instant::now();
		delta
	}
}
