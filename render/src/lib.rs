mod camera_3d;
mod depth_texture;
mod entry;
mod eye_dome;
mod lookup;
mod mesh;
mod point;
mod point_cloud;
mod render_pass;
mod state;
mod texture;
mod vertex_2d;
mod window;
mod lines;


pub use camera_3d::*;
pub use depth_texture::*;
pub use entry::*;
pub use eye_dome::*;
pub use lookup::*;
use math::Vector;
pub use mesh::*;
pub use point::*;
pub use point_cloud::*;
pub use render_pass::*;
pub use state::*;
pub use texture::*;
pub use vertex_2d::*;
pub use window::*;
pub use lines::*;
pub use egui;


pub trait RenderEntry<State> {
	fn background(&self) -> Vector<3, f32>;


	fn render<'a>(&'a mut self, state: &'a State, render_pass: &mut RenderPass<'a>);


	fn post_process<'a>(&'a mut self, state: &'a State, render_pass: &mut RenderPass<'a>);
}


pub trait Has<T> {
	fn get(&self) -> &T;
}


impl<T> Has<T> for T {
	fn get(&self) -> &T { self }
}
