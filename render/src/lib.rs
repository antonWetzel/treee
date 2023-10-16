mod camera_3d;
mod depth_texture;
mod entry;
mod eye_dome;
mod lookup;
mod point;
mod point_cloud;
mod state;
mod texture;
mod ui;
mod vertex_2d;
mod window;

pub use camera_3d::*;
pub use entry::*;
pub use eye_dome::*;
pub use lookup::*;
pub use point::*;
pub use point_cloud::*;
pub use state::*;
pub use texture::*;
pub use ui::*;
pub use vertex_2d::*;
pub use window::*;

use depth_texture::*;

pub trait Renderable {
	fn render<'a>(&'a self, render_pass: RenderPass<'a>) -> RenderPass<'a>;

	fn post_process<'a>(&'a self, render_pass: RenderPass<'a>) -> RenderPass<'a>;
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

pub trait ChainExtension
where
	Self: Sized,
{
	fn next<O>(self, f: impl FnOnce(Self) -> O) -> O {
		f(self)
	}
}

impl ChainExtension for RenderPass<'_> {}
impl ChainExtension for UIPass<'_> {}
impl ChainExtension for PointCloudPass<'_> {}
