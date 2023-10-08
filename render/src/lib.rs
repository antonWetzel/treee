mod camera_3d;
mod depth_texture;
mod eye_dome;
mod game;
mod pipeline;
mod point;
mod point_cloud;
mod state;
mod window;

pub use camera_3d::*;
pub use game::*;
pub use pipeline::*;
pub use point::*;
pub use point_cloud::*;
pub use state::*;
pub use window::*;

use depth_texture::*;

pub trait Renderable<State> {
	fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>, state: &'a State);
}

pub type Device = wgpu::Device;

pub trait Has<T> {
	fn get(&self) -> &T;
}

impl<T> Has<T> for T {
	fn get(&self) -> &T {
		self
	}
}
