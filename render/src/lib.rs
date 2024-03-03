mod camera_3d;
mod depth_texture;
mod entry;
mod eye_dome;
mod lines;
mod lookup;
mod mesh;
mod point;
mod point_cloud;
mod render_pass;
mod state;
mod texture;
mod vertex_2d;
mod window;

use std::{ops::Deref, sync::Arc};

pub use camera_3d::*;
pub use depth_texture::*;
pub use egui;
pub use entry::*;
pub use eye_dome::*;
pub use lines::*;
pub use lookup::*;
pub use mesh::*;
pub use point::*;
pub use point_cloud::*;
pub use render_pass::*;
pub use state::*;
pub use texture::*;
pub use vertex_2d::*;
pub use window::*;

use math::Vector;

pub trait RenderEntry<TState> {
	fn background(&self) -> Vector<3, f32>;
	fn render<'a>(&'a mut self, state: &'a TState, render_pass: &mut RenderPass<'a>);
	fn post_process<'a>(&'a mut self, state: &'a TState, render_pass: &mut RenderPass<'a>);
}

pub trait Has<T> {
	fn get(&self) -> &T;
}

impl<X, T> Has<X> for Arc<T>
where
	T: Has<X>,
{
	fn get(&self) -> &X {
		self.deref().get()
	}
}
