use math::Vector;

use super::*;

pub type Event = winit::event::WindowEvent;

pub trait Entry {
	fn raw_event(&mut self, event: &Event) -> bool;

	fn close_window(&mut self, window_id: WindowId);

	fn resize_window(&mut self, window_id: WindowId, size: Vector<2, u32>);

	fn key_changed(&mut self, window_id: WindowId, key: input::KeyCode, key_state: input::State);

	fn mouse_button_changed(&mut self, window_id: WindowId, button: input::MouseButton, button_state: input::State);

	fn mouse_wheel(&mut self, delta: f32);

	fn mouse_moved(&mut self, window_id: WindowId, position: Vector<2, f32>);

	fn time(&mut self);

	fn render(&mut self, window_id: WindowId);

	fn modifiers_changed(&mut self, modifiers: input::Modifiers);

	fn exit(&self) -> bool;
}

pub type ControlFlow = winit::event_loop::ControlFlow;
