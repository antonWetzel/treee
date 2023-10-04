mod camera;
mod depth_texture;
mod game;
mod point;
mod state;
mod window;

pub mod gpu;

pub use camera::*;
pub use game::*;
pub use point::*;
pub use state::*;
pub use window::*;

use depth_texture::*;

pub trait Renderable {
	fn render<'a, 'b: 'a>(&'a self, render_pass: &mut RenderPass<'a>, state: &'b State);
}

pub type Device = wgpu::Device;
