mod camera;
mod loaded_manager;
mod lod;
mod reader;
mod segment;
mod state;
mod tree;
mod world;

use nalgebra as na;
use pollster::FutureExt;
use state::State;
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("no file")]
	NoFile,
	#[error("{0}")]
	RenderError(#[from] render::RenderError),
}

pub type EventLoop = winit::event_loop::EventLoop<()>;

pub fn run(event_loop: &mut EventLoop) -> Result<(), Error> {
	let mut world = world::World::new(event_loop).block_on()?;

	event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
	event_loop.run_on_demand(|event, event_loop| {
		match event {
			winit::event::Event::WindowEvent { event, window_id } => {
				if world.raw_event(&event) {
					return;
				}
				match event {
					winit::event::WindowEvent::CloseRequested => world.close_window(window_id),
					winit::event::WindowEvent::Resized(size) => {
						world.resize_window(window_id, [size.width, size.height].into())
					},
					winit::event::WindowEvent::ScaleFactorChanged { .. } => todo!(),
					winit::event::WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
						winit::keyboard::PhysicalKey::Code(key) => world.key_changed(window_id, key, event.state),
						winit::keyboard::PhysicalKey::Unidentified(_) => {},
					},
					winit::event::WindowEvent::MouseInput { state: button_state, button, .. } => {
						world.mouse_button_changed(window_id, (button).into(), button_state)
					},
					winit::event::WindowEvent::MouseWheel { delta, .. } => {
						let delta = match delta {
							winit::event::MouseScrollDelta::LineDelta(_, y) => -y,
							winit::event::MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
						};
						world.mouse_wheel(delta)
					},
					winit::event::WindowEvent::CursorMoved { position, .. } => {
						let position = na::vector![position.x as f32, position.y as f32].into();
						world.mouse_moved(window_id, position)
					},
					winit::event::WindowEvent::ModifiersChanged(modifiers) => {
						world.modifiers_changed(modifiers.state())
					},
					winit::event::WindowEvent::RedrawRequested => {
						world.time();
						world.render(window_id);
						world.request_redraw();
					},
					_ => {},
				}
			},
			_ => {},
		}

		if world.exit() {
			event_loop.exit();
		}
	})?;

	Ok(())
}
