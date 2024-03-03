use std::sync::Arc;

use math::Vector;

use crate::State;

pub trait CustomState {
	type Tree;
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
