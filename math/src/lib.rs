mod angle;
mod mat;
mod matrix;
mod projection;
mod quaternion;
mod requirements;
mod transform;
mod vector;

pub use angle::*;
pub use mat::*;
pub use matrix::*;
pub use projection::*;
pub use quaternion::*;
pub use requirements::*;
pub use transform::*;
pub use vector::*;

use std::ops::Range;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct Dimension(pub usize);

impl From<Dimension> for usize {
	fn from(val: Dimension) -> Self {
		val.0
	}
}

pub const DIM_0: Dimension = Dimension(0);
pub const X: Dimension = Dimension(0);
pub const Y: Dimension = Dimension(1);
pub const Z: Dimension = Dimension(2);
pub const W: Dimension = Dimension(3);

pub struct Dimension2D(pub usize, pub usize);

impl From<Dimension2D> for (usize, usize) {
	fn from(val: Dimension2D) -> Self {
		(val.0, val.1)
	}
}

impl std::ops::Add for Dimension {
	type Output = Dimension2D;

	fn add(self, rhs: Self) -> Self::Output {
		Dimension2D(self.0, rhs.0)
	}
}

pub struct Dimensions(pub Range<usize>);

impl Iterator for Dimensions {
	type Item = Dimension;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(Dimension)
	}
}

impl Dimension {
	pub const fn to(self, to: Dimension) -> Dimensions {
		Dimensions(self.0..(to.0 + 1))
	}

	pub const fn next(self, max: Dimension) -> Dimension {
		Self((self.0 + 1) % (max.0 + 1))
	}

	pub const fn previous(self, max: Dimension) -> Dimension {
		Self((self.0 + max.0) % (max.0 + 1))
	}
}
