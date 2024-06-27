mod camera_3d;
mod depth_texture;
mod eye_dome;
mod lines;
mod lookup;
mod mesh;
mod point;
mod point_cloud;
mod state;
mod texture;
mod vertex_2d;
mod window;

pub use camera_3d::*;
pub use depth_texture::*;
pub use eye_dome::*;
pub use lines::*;
pub use lookup::*;
pub use mesh::*;
pub use point::*;
pub use point_cloud::*;
pub use state::*;
pub use texture::*;
pub use vertex_2d::*;
pub use window::*;

pub struct RenderPass<'a>(wgpu::RenderPass<'a>);

impl<'a> RenderPass<'a> {}

impl<'a> std::ops::Deref for RenderPass<'a> {
	type Target = wgpu::RenderPass<'a>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<'a> std::ops::DerefMut for RenderPass<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<'a> RenderPass<'a> {
	pub fn new(render_pass: wgpu::RenderPass<'a>) -> Self {
		Self(render_pass)
	}
}

pub type CommandEncoder = wgpu::CommandEncoder;
