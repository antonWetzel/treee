use math::Vector;
use serde::{ Deserialize, Serialize };

use crate::camera;


#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Mode {
	Normal { threshold: f32 },
	Auto { threshold: f32 },
	Level { target: usize, max: usize },
}


#[derive(Clone, Copy)]
pub enum Checker {
	Normal { threshold: f32 },
	Level { current: usize, target: usize },
}


impl Mode {
	pub fn change_detail(&mut self, amount: f32) {
		match self {
			Mode::Normal { threshold, .. } => *threshold *= 1.0 + amount / 10.0,
			Mode::Auto { .. } => { },
			Mode::Level { target, max } => {
				if amount < 0.0 {
					*target -= (*target > 0) as usize
				} else {
					*target += (*target < *max) as usize
				}
			},
		}
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


	pub fn should_render(self, corner: Vector<3, f32>, size: f32, camera: &camera::Camera) -> bool {
		match self {
			Self::Level { current, target } => current >= target,
			Self::Normal { threshold } => {
				let rad = size * 0.5;
				let pos = corner + Vector::new([rad, rad, rad]);
				let dist = (camera.position() - pos).length() - (3.0 * rad * rad).sqrt();
				let dist = if dist < 0.0 { 0.0 } else { dist };
				(dist / (size * size)) > threshold
			},
		}
	}
}
