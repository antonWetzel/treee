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
