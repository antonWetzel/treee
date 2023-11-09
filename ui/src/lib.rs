mod area;
mod button;
mod image;
mod popup;
mod ui_collection;

pub use area::*;
pub use button::*;
pub use image::*;
pub use popup::*;

use math::{Vector, X, Y};
use render::Has;

pub trait Event: Copy {}

impl<T> Event for T where T: Copy {}

#[derive(Clone, Copy)]
pub struct Size {
	pub absolute: f32,
	pub relative_width: f32,
	pub relative_height: f32,
}

impl Size {
	pub const FULL_RECT: Vector<2, Size> = Vector::new([
		Size {
			absolute: 0.0,
			relative_width: 1.0,
			relative_height: 0.0,
		},
		Size {
			absolute: 0.0,
			relative_width: 0.0,
			relative_height: 1.0,
		},
	]);

	pub fn new() -> Self {
		Self {
			absolute: 0.0,
			relative_width: 0.0,
			relative_height: 0.0,
		}
	}

	pub fn map(&self, base: f32, size: Vector<2, f32>) -> f32 {
		base + self.absolute + self.relative_width * size[X] + self.relative_height * size[Y]
	}

	pub fn abs(mut self, absolute: f32) -> Self {
		self.absolute += absolute;
		self
	}

	pub fn height(mut self, relative: f32) -> Self {
		self.relative_height += relative;
		self
	}

	pub fn width(mut self, relative: f32) -> Self {
		self.relative_width += relative;
		self
	}
}

#[derive(Clone, Copy)]
pub struct Anchor {
	pub position: Vector<2, Size>,
	pub size: Vector<2, Size>,
}

impl Anchor {
	pub fn new(position: Vector<2, Size>, size: Vector<2, Size>) -> Self {
		Self { position, size }
	}

	pub fn square(position: Vector<2, Size>, size: Size) -> Self {
		Self::new(position, [size, size].into())
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
	pub position: Vector<2, f32>,
	pub size: Vector<2, f32>,
}

impl Rect {
	pub fn inside(self, position: Vector<2, f32>) -> bool {
		let max = self.position + self.size;
		self.position[X] <= position[X]
			&& position[X] < max[X]
			&& self.position[Y] <= position[Y]
			&& position[Y] < max[Y]
	}
}

pub trait Element: render::UIElement {
	type Event: Event;

	fn inside(&self, position: Vector<2, f32>) -> bool;

	fn resize(&mut self, state: &(impl Has<render::State> + Has<render::UIState>), rect: Rect);
	#[allow(unused)]
	fn click(&mut self, position: Vector<2, f32>) -> Option<Self::Event> {
		None
	}
	#[allow(unused)]
	fn hover(&mut self, position: Vector<2, f32>) -> Option<Self::Event> {
		None
	}
}
