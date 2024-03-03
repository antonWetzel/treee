use std::sync::Arc;

use math::Vector;
use render::Window;

use crate::{camera::Camera, lod, State};

pub const DEFAULT_BACKGROUND: Vector<3, f32> = Vector::new([0.1, 0.2, 0.3]);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LookupName {
	Warm,
	Cold,
	Turbo,
}

impl LookupName {
	pub fn data(self) -> &'static [u8] {
		match self {
			Self::Warm => include_bytes!("../assets/grad_warm.png"),
			Self::Cold => include_bytes!("../assets/grad_cold.png"),
			Self::Turbo => include_bytes!("../assets/grad_turbo.png"),
		}
	}
}
pub trait Scene: Sized {
	fn update(&mut self, view_checker: lod::Checker, camera: &Camera);
	fn render<'a>(&'a self, state: &'a State, tree: &'a Tree<Self>, render_pass: &mut render::RenderPass<'a>);
}
pub struct Tree<T> {
	pub context: TreeContext,
	pub scene: T,
}

impl<T> std::ops::Deref for Tree<T> {
	type Target = TreeContext;

	fn deref(&self) -> &Self::Target {
		&self.context
	}
}

impl<T: Scene> render::RenderEntry<State> for Tree<T> {
	fn background(&self) -> Vector<3, f32> {
		self.context.background
	}

	fn render<'a>(&'a mut self, state: &'a State, render_pass: &mut render::RenderPass<'a>) {
		self.scene.render(state, self, render_pass)
	}

	fn post_process<'a>(&'a mut self, _state: &'a State, render_pass: &mut render::RenderPass<'a>) {
		if self.context.eye_dome_active {
			render_pass.render(&self.context.eye_dome, ());
		}
	}
}

impl<T> Tree<T> {
	pub fn new(state: Arc<State>, property: (String, String, u32), window: &Window, scene: T) -> Self {
		let lookup_name = LookupName::Warm;

		Self {
			context: TreeContext {
				background: DEFAULT_BACKGROUND,
				camera: Camera::new(&state, window.get_aspect()),
				lookup_name,
				lookup: render::Lookup::new_png(&state, lookup_name.data(), property.2),
				environment: render::PointCloudEnvironment::new(&state, u32::MIN, u32::MAX, 1.0),
				eye_dome: render::EyeDome::new(&state, window.config(), window.depth_texture(), 0.7),
				eye_dome_active: true,
				voxels_active: false,

				property,
			},

			scene,
		}
	}
}

pub struct TreeContext {
	pub camera: crate::camera::Camera,

	pub lookup: render::Lookup,
	pub environment: render::PointCloudEnvironment,
	pub background: Vector<3, f32>,

	pub lookup_name: LookupName,
	pub eye_dome: render::EyeDome,
	pub eye_dome_active: bool,
	pub voxels_active: bool,
	pub property: (String, String, u32),
}

impl TreeContext {
	pub fn update_lookup(&mut self, state: &State) {
		self.lookup = render::Lookup::new_png(state, self.lookup_name.data(), self.property.2);
	}
}
