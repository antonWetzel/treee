use super::State;
use math::Vector;
use std::collections::HashSet;


#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MouseButton {
	Left,
	Right,
	Middle,
	Backward,
	Forward,
	Unknown,
}


pub type MouseButtonState = winit::event::ElementState;


impl From<winit::event::MouseButton> for MouseButton {
	fn from(value: winit::event::MouseButton) -> Self {
		match value {
			winit::event::MouseButton::Left => MouseButton::Left,
			winit::event::MouseButton::Right => MouseButton::Right,
			winit::event::MouseButton::Middle => MouseButton::Middle,
			winit::event::MouseButton::Back => MouseButton::Backward,
			winit::event::MouseButton::Forward => MouseButton::Forward,
			winit::event::MouseButton::Other(_) => MouseButton::Unknown,
		}
	}
}


pub struct Mouse {
	pub(crate) pressed: HashSet<MouseButton>,
	pub(crate) position: Vector<2, f32>,
}


impl Mouse {
	pub fn new() -> Self {
		Self {
			pressed: HashSet::new(),
			position: Vector::default(),
		}
	}


	pub fn update(&mut self, button: MouseButton, button_state: State) {
		match button_state {
			winit::event::ElementState::Pressed => self.pressed.insert(button),
			winit::event::ElementState::Released => self.pressed.remove(&button),
		};
	}


	pub fn delta(&mut self, position: Vector<2, f32>) -> Vector<2, f32> {
		let delta = position - self.position;
		self.position = position;
		delta
	}


	pub fn pressed(&self, button: MouseButton) -> bool {
		self.pressed.contains(&button)
	}


	pub fn position(&self) -> Vector<2, f32> {
		self.position
	}
}


impl Default for Mouse {
	fn default() -> Self {
		Self::new()
	}
}
