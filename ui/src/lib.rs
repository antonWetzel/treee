mod area;
mod button;
mod hide;
mod image;
mod popup;
mod relative;
mod slider;
mod split;
mod text;
mod ui_collection;

pub use area::*;
pub use button::*;
pub use hide::*;
pub use image::*;
pub use popup::*;
pub use relative::*;
pub use slider::*;
pub use split::*;
pub use text::*;

use math::{Vector, X, Y};
use render::Has;

pub struct Horizontal;
pub struct Vertical;

#[derive(Clone, Copy)]
pub struct Length {
	pub absolute: f32,
	pub relative_width: f32,
	pub relative_height: f32,
}

impl Length {
	pub const FULL_RECT: Vector<2, Length> = Vector::new([
		Length {
			absolute: 0.0,
			relative_width: 1.0,
			relative_height: 0.0,
		},
		Length {
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
}

impl Default for Length {
	fn default() -> Self {
		Self::new()
	}
}

impl std::ops::Add for Length {
	type Output = Length;

	fn add(self, rhs: Self) -> Self::Output {
		Self {
			absolute: self.absolute + rhs.absolute,
			relative_width: self.relative_width + rhs.relative_width,
			relative_height: self.relative_height + rhs.relative_height,
		}
	}
}

#[derive(Clone, Copy)]
pub struct Anchor {
	pub min: Vector<2, Length>,
	pub max: Vector<2, Length>,
}

impl Anchor {
	pub fn new(position: Vector<2, Length>, size: Vector<2, Length>) -> Self {
		Self { min: position, max: position + size }
	}

	pub fn square(position: Vector<2, Length>, size: Length) -> Self {
		Self::new(position, [size, size].into())
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
	pub min: Vector<2, f32>,
	pub max: Vector<2, f32>,
}

impl Rect {
	pub fn inside(self, position: Vector<2, f32>) -> bool {
		self.min[X] <= position[X]
			&& position[X] < self.max[X]
			&& self.min[Y] <= position[Y]
			&& position[Y] < self.max[Y]
	}

	pub fn merge(self, other: Rect) -> Rect {
		Rect {
			min: [self.min[X].min(other.min[X]), self.min[Y].min(other.min[Y])].into(),
			max: [self.max[X].max(other.max[X]), self.max[Y].max(other.max[Y])].into(),
		}
	}

	pub fn size(self) -> Vector<2, f32> {
		self.max - self.min
	}
}

pub struct Empty<E> {
	pub rect: Rect,
	phantom: std::marker::PhantomData<E>,
}

impl<E> Empty<E> {
	pub fn new() -> Self {
		let position = [0.0, 0.0].into();
		let size = [16.0, 16.0].into();
		Self {
			rect: Rect { min: position, max: position + size },
			phantom: std::marker::PhantomData,
		}
	}
}

impl<E> Default for Empty<E> {
	fn default() -> Self {
		Self::new()
	}
}

impl<E> render::UIElement for Empty<E> {
	fn render<'a>(&'a self, _ui_pass: &mut render::UIPass<'a>) {}
	fn collect<'a>(&'a self, _collector: &mut render::UICollector<'a>) {}
}

impl<E> Element for Empty<E> {
	type Event = E;

	fn resize(&mut self, _state: &impl State, rect: Rect) {
		self.rect = rect
	}

	fn bounding_rect(&self) -> Rect {
		self.rect
	}

	fn inside(&self, position: Vector<2, f32>) -> bool {
		self.rect.inside(position)
	}

	fn click(&mut self, _state: &impl State, _position: Vector<2, f32>) -> Option<Self::Event> {
		None
	}
	fn release(&mut self, _position: Vector<2, f32>) -> bool {
		false
	}

	fn hover(&mut self, _state: &impl State, _position: Vector<2, f32>, _pressed: bool) -> Option<Self::Event> {
		None
	}
}

pub trait State: Has<render::State> + Has<render::UIState> {}
impl<T: Has<render::State> + Has<render::UIState>> State for T {}

pub trait Element: render::UIElement {
	type Event;

	fn inside(&self, position: Vector<2, f32>) -> bool;

	fn resize(&mut self, state: &impl State, rect: Rect);

	fn bounding_rect(&self) -> Rect;

	fn click(&mut self, state: &impl State, position: Vector<2, f32>) -> Option<Self::Event>;
	fn release(&mut self, position: Vector<2, f32>) -> bool;
	fn hover(&mut self, state: &impl State, position: Vector<2, f32>, pressed: bool) -> Option<Self::Event>;
}

#[macro_export]
macro_rules! length {
	($abs:expr , w $w:expr , h $h:expr) => {
		$crate::Length {
			absolute: $abs,
			relative_width: $w,
			relative_height: $h,
		}
	};
	($abs:expr , w $w:expr) => {
		$crate::Length {
			absolute: $abs,
			relative_width: $w,
			relative_height: 0.0,
		}
	};
	($abs:expr , h $h:expr) => {
		$crate::Length {
			absolute: $abs,
			relative_width: 0.0,
			relative_height: $h,
		}
	};
	(w $w:expr , h $h:expr) => {
		$crate::Length {
			absolute: 0.0,
			relative_width: $w,
			relative_height: $h,
		}
	};
	($abs:expr) => {
		$crate::Length {
			absolute: $abs,
			relative_width: 0.0,
			relative_height: 0.0,
		}
	};
	(w $w:expr) => {
		$crate::Length {
			absolute: 0.0,
			relative_width: $w,
			relative_height: 0.0,
		}
	};
	(h $h:expr) => {
		$crate::Length {
			absolute: 0.0,
			relative_width: 0.0,
			relative_height: $h,
		}
	};
	() => {
		$crate::Length {
			absolute: 0.0,
			relative_width: 0.0,
			relative_height: 0.0,
		}
	};
}
