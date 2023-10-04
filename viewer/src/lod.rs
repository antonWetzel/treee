use math::Vector;

use crate::camera;

pub enum Mode {
	Normal { threshold: f32 },
	Level { target: usize, max: usize },
}

pub enum Checker {
	Normal { threshold: f32 },
	Level { current: usize, target: usize },
}

impl Mode {
	pub fn increase_detail(&mut self) {
		match self {
			Mode::Normal { threshold, .. } => {
				*threshold *= 1.2;
				println!("Threshold: {}", threshold);
			},
			Mode::Level { target, max } => {
				if *target < *max {
					*target += 1;
				}
				println!("Target level: {}", target);
			},
		}
	}

	pub fn decrese_detail(&mut self) {
		match self {
			Mode::Normal { threshold, .. } => {
				*threshold /= 1.2;
				println!("Threshold: {}", threshold);
			},
			Mode::Level { target, .. } => {
				if *target > 0 {
					*target -= 1;
				}
				println!("Target level: {}", target);
			},
		}
	}
}

impl Checker {
	pub fn new(mode: &Mode) -> Self {
		match mode {
			&Mode::Normal { threshold } => Self::Normal { threshold },
			&Mode::Level { target, .. } => Self::Level { current: 0, target },
		}
	}
	pub fn level_down(&mut self) {
		match self {
			Self::Level { current, .. } => *current += 1,
			_ => {},
		}
	}

	pub fn level_up(&mut self) {
		match self {
			Self::Level { current, .. } => *current -= 1,
			_ => {},
		}
	}

	pub fn should_render(&self, corner: Vector<3, f32>, size: f32, camera: &camera::Camera) -> bool {
		match self {
			&Self::Level { current, target } => current >= target,
			&Self::Normal { threshold } => {
				let rad = size * 0.5;
				let pos = corner + Vector::new([rad, rad, rad]);
				let dist = (camera.position() - pos).length() - (3.0 * rad * rad).sqrt();
				let dist = if dist < 0.0 { 0.0 } else { dist };
				(dist / (size * size)) > threshold
			},
		}
	}
}
