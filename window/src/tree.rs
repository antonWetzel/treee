use std::sync::Arc;

use math::Vector;
use render::Window;

use crate::{camera::Camera, State};

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
pub struct Tree<T> {
	pub camera: crate::camera::Camera,

	pub lookup: render::Lookup,
	pub environment: render::PointCloudEnvironment,
	pub background: Vector<3, f32>,

	pub lookup_name: LookupName,
	pub eye_dome: render::EyeDome,
	pub eye_dome_active: bool,
	pub voxels_active: bool,
	pub scene: T,

	pub property: (String, String, u32),
}

impl<T> Tree<T> {
	pub fn new(state: Arc<State>, property: (String, String, u32), window: &Window, scene: T) -> Self {
		let lookup_name = LookupName::Warm;

		Self {
			background: DEFAULT_BACKGROUND,
			camera: Camera::new(&state, window.get_aspect()),

			lookup_name,
			lookup: render::Lookup::new_png(&state, lookup_name.data(), property.2),
			environment: render::PointCloudEnvironment::new(&state, u32::MIN, u32::MAX, 1.0),
			eye_dome: render::EyeDome::new(&state, window.config(), window.depth_texture(), 0.7),
			scene,
			eye_dome_active: true,
			voxels_active: false,

			property,
		}
	}
}
