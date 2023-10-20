use math::Vector;

use super::*;

// todo: pass window_target to allow the creation of the windows
pub trait RenderEntry {
	fn close_window(&mut self, window_id: WindowId) -> ControlFlow;
	fn resize_window(&mut self, window_id: WindowId, size: Vector<2, u32>) -> ControlFlow;
	fn key_changed(&mut self, window_id: WindowId, key: input::KeyCode, key_state: input::State) -> ControlFlow;
	fn mouse_button_changed(
		&mut self,
		window_id: WindowId,
		button: input::MouseButton,
		button_state: input::State,
	) -> ControlFlow;
	fn mouse_wheel(&mut self, delta: f32) -> ControlFlow;
	fn mouse_moved(&mut self, window_id: WindowId, position: Vector<2, f32>) -> ControlFlow;
	fn time(&mut self) -> ControlFlow;
	fn render(&mut self, window_id: WindowId);
	fn modifiers_changed(&mut self, modifiers: input::Modifiers);
}

pub type ControlFlow = winit::event_loop::ControlFlow;

pub type RenderPass<'a> = wgpu::RenderPass<'a>;
