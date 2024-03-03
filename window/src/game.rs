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

	pub fn take_egui_input(&mut self) -> RawInput {
		self.window.egui_winit.take_egui_input(&self.window.window)
	}

	pub fn time_delta(&mut self, delta: std::time::Duration) {
		let mut direction: Vector<2, f32> = [0.0, 0.0].into();
		if self.keyboard.pressed(input::KeyCode::KeyD) || self.keyboard.pressed(input::KeyCode::ArrowRight) {
			direction[X] += 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyS) || self.keyboard.pressed(input::KeyCode::ArrowDown) {
			direction[Y] += 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyA) || self.keyboard.pressed(input::KeyCode::ArrowLeft) {
			direction[X] -= 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyW) || self.keyboard.pressed(input::KeyCode::ArrowUp) {
			direction[Y] -= 1.0;
		}
		let l = direction.length();
		if l > 0.0 {
			direction *= 10.0 * delta.as_secs_f32() / l;
			self.tree.context.camera.movement(direction, &self.state);
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

impl<T: CustomState> render::Entry for Game<T> {
	fn raw_event(&mut self, event: &render::Event) -> bool {
		let response = self.window.window_event(event);
		response.consumed
	}

	fn render(&mut self, _window_id: render::WindowId) {
		todo!("Not yet standalone usable")
	}

	fn resize_window(&mut self, _window_id: render::WindowId, size: Vector<2, u32>) {
		self.paused = size[X] == 0 || size[Y] == 0;
		if self.paused {
			return;
		}

		self.window.resized(&self.state);
		self.tree
			.context
			.camera
			.cam
			.set_aspect(self.window.get_aspect());
		self.tree.context.camera.gpu = render::Camera3DGPU::new(
			&self.state,
			&self.tree.camera.cam,
			&self.tree.camera.transform,
		);
		self.tree
			.context
			.eye_dome
			.update_depth(&self.state, self.window.depth_texture());
	}

	fn request_redraw(&mut self) {
		self.window.request_redraw();
	}

	fn close_window(&mut self, _window_id: render::WindowId) {
		self.quit = true;
	}

	fn time(&mut self) {
		let delta = self.time.elapsed();
		self.time_delta(delta);
	}

	fn key_changed(&mut self, _window_id: render::WindowId, key: input::KeyCode, key_state: input::State) {
		self.keyboard.update(key, key_state);
	}

	fn modifiers_changed(&mut self, modifiers: input::Modifiers) {
		self.keyboard.update_modifiers(modifiers);
	}

	fn mouse_wheel(&mut self, delta: f32) {
		self.tree.context.camera.scroll(delta, &self.state);
	}

	fn mouse_button_changed(
		&mut self,
		_window_id: render::WindowId,
		button: input::MouseButton,
		button_state: input::State,
	) {
		self.mouse.update(button, button_state);
	}

	fn mouse_moved(&mut self, _window_id: render::WindowId, position: Vector<2, f32>) {
		let delta = self.mouse.delta(position);
		if self.mouse.pressed(input::MouseButton::Left) {
			self.tree.context.camera.rotate(delta, &self.state);
		}
	}

	fn exit(&self) -> bool {
		self.quit
	}
}
