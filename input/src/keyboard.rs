use super::State;
use std::collections::HashSet;

pub struct Keyboard {
	pub(crate) pressed: HashSet<KeyCode>,
	modifiers: Modifiers,
}

impl Keyboard {
	pub fn new() -> Self {
		Self {
			pressed: HashSet::new(),
			modifiers: Modifiers::default(),
		}
	}

	pub fn update(&mut self, key: KeyCode, key_state: State) {
		match key_state {
			winit::event::ElementState::Pressed => {
				self.pressed.insert(key);
			},
			winit::event::ElementState::Released => {
				self.pressed.remove(&key);
			},
		}
	}

	pub fn update_modifiers(&mut self, modifiers: Modifiers) {
		self.modifiers = modifiers;
	}

	pub fn pressed(&self, key: KeyCode) -> bool {
		return self.pressed.contains(&key);
	}
}

pub type KeyCode = winit::event::VirtualKeyCode; //todo: maybe physical is better
pub type Modifiers = winit::event::ModifiersState;
