mod camera_3d;
mod depth_texture;
mod entry;
mod eye_dome;
mod lookup;
mod point;
mod point_cloud;
mod render_pass;
mod state;
mod texture;
mod ui;
mod vertex_2d;
mod window;

pub use camera_3d::*;
pub use depth_texture::*;
pub use entry::*;
pub use eye_dome::*;
pub use lookup::*;
pub use point::*;
pub use point_cloud::*;
pub use render_pass::*;
pub use state::*;
pub use texture::*;
pub use ui::*;
pub use vertex_2d::*;
pub use window::*;

pub trait RenderEntry {
	fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>);

	fn post_process<'a>(&'a self, render_pass: &mut RenderPass<'a>);
}

pub trait Has<T> {
	fn get(&self) -> &T;
}

impl<T> Has<T> for T {
	fn get(&self) -> &T {
		self
	}
}
