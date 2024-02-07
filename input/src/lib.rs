mod keyboard;
mod mouse;

pub use keyboard::*;
pub use mouse::*;

pub type State = winit::event::ElementState;
