use math::Vector;

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
