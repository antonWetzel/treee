use nalgebra as na;
use serde::{Deserialize, Serialize};

use crate::camera;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Mode {
	Normal { threshold: f32 },
	Auto { threshold: f32, target: f32 },
	Level { target: usize, max: usize },
}

#[derive(Clone, Copy)]
pub enum Checker {
	Normal { threshold: f32 },
	Level { current: usize, target: usize },
}

impl Mode {
	pub fn new_auto() -> Self {
		Self::Auto { threshold: 1.0, target: 60.0 }
	}

	pub fn new_normal() -> Self {
		Self::Normal { threshold: 1.0 }
	}

	pub fn new_level(max_level: usize) -> Self {
		Self::Level { target: 0, max: max_level }
	}
}

impl Checker {
	pub fn new(mode: &Mode) -> Self {
		match *mode {
			Mode::Normal { threshold } | Mode::Auto { threshold, .. } => Self::Normal { threshold },
			Mode::Level { target, .. } => Self::Level { current: 0, target },
		}
	}

	pub fn level_down(self) -> Self {
		match self {
			Self::Level { current, target } => Self::Level { current: current + 1, target },
			_ => self,
		}
	}

	pub fn should_render(self, corner: na::Point3<f32>, size: f32, camera: &camera::Camera) -> bool {
		match self {
			Self::Level { current, target } => current >= target,
			Self::Normal { threshold } => {
				let rad = size * 0.5;
				let pos = corner + na::vector![rad, rad, rad];
				let dist = (camera.position() - pos).norm() - (3.0 * rad * rad).sqrt();
				// let dist = if dist < 0.0 { 0.0 } else { dist };
				(dist / (size * size)) > threshold
			},
		}
	}
}
