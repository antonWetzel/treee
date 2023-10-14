mod camera_3d;
mod depth_texture;
mod eye_dome;
mod game;
mod lookup;
mod point;
mod point_cloud;
mod state;
mod ui;
mod window;

pub use camera_3d::*;
pub use eye_dome::*;
pub use game::*;
pub use lookup::*;
pub use point::*;
pub use point_cloud::*;
pub use state::*;
pub use ui::*;
pub use window::*;

use depth_texture::*;

pub trait Renderable<State> {
	fn render<'a>(&'a self, render_pass: RenderPass<'a>, state: &'a State) -> RenderPass<'a>;

	fn post_process<'a>(&'a self, render_pass: RenderPass<'a>, state: &'a State) -> RenderPass<'a>;
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
