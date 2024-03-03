use std::sync::Arc;

use math::{Vector, X, Y};
use render::{egui::RawInput, Window};

use crate::{tree::Tree, State};

pub trait CustomState {
	type Scene;
}

pub struct Game<TCustomState: CustomState> {
	pub window: render::Window,
	pub tree: Tree<TCustomState::Scene>,
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
	pub fn new(
		window: render::Window,
		tree: Tree<TCustomState::Scene>,
		state: Arc<State>,
		custom_state: TCustomState,
	) -> Self {
		Self {
			window,
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
		self.tree.context.camera.cam.set_aspect(window.get_aspect());
		self.tree
			.context
			.eye_dome
			.update_depth(&self.state, &window.depth_texture());
		self.tree.context.camera.gpu = render::Camera3DGPU::new(
			&self.state,
			&self.tree.camera.cam,
			&self.tree.camera.transform,
		);
	}

	pub fn take_egui_input(&mut self) -> RawInput {
		self.window.egui_winit.take_egui_input(&self.window.window)
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
