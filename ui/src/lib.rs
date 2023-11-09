mod area;
mod button;
mod image;
mod popup;
mod split;
mod ui_collection;

pub use area::*;
pub use button::*;
pub use image::*;
pub use popup::*;
pub use split::*;

use math::{Vector, X, Y};
use render::Has;

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

	pub fn abs(mut self, abs: f32) -> Self {
		self.absolute += abs;
		self
	}

	pub fn h(mut self, rel: f32) -> Self {
		self.relative_height += rel;
		self
	}

	pub fn w(mut self, rel: f32) -> Self {
		self.relative_width += rel;
		self
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
}

impl<E> Element for Empty<E> {
	type Event = E;

	fn resize(&mut self, _state: &(impl Has<render::State> + Has<render::UIState>), rect: Rect) {
		self.rect = rect
	}

	fn bounding_rect(&self) -> Rect {
		self.rect
	}

	fn inside(&self, position: Vector<2, f32>) -> bool {
		self.rect.inside(position)
	}

	fn click(&mut self, _position: Vector<2, f32>) -> Option<Self::Event> {
		None
	}

	fn hover(&mut self, _position: Vector<2, f32>) -> Option<Self::Event> {
		None
	}
}

pub trait Element: render::UIElement {
	type Event;

	fn inside(&self, position: Vector<2, f32>) -> bool;

	fn resize(&mut self, state: &(impl Has<render::State> + Has<render::UIState>), rect: Rect);

	fn bounding_rect(&self) -> Rect;

	fn click(&mut self, position: Vector<2, f32>) -> Option<Self::Event>;
	fn hover(&mut self, position: Vector<2, f32>) -> Option<Self::Event>;
}

pub struct RelHeight<Base: Element> {
	base: Base,
	scale: f32,
}

impl<Base: Element> RelHeight<Base> {
	pub fn new(base: Base, scale: f32) -> Self {
		Self { base, scale }
	}

	pub fn square(base: Base) -> Self {
		Self { base, scale: 1.0 }
	}
}

impl<Base: Element> render::UIElement for RelHeight<Base> {
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.base.render(ui_pass);
	}

	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		self.base.collect(collector);
	}
}

impl<Base: Element> Element for RelHeight<Base> {
	type Event = Base::Event;

	fn bounding_rect(&self) -> crate::Rect {
		self.base.bounding_rect()
	}
	fn click(&mut self, position: math::Vector<2, f32>) -> Option<Self::Event> {
		if self.inside(position) {
			return self.base.click(position);
		}
		None
	}

	fn hover(&mut self, position: math::Vector<2, f32>) -> Option<Self::Event> {
		self.base.hover(position)
	}

	fn inside(&self, position: math::Vector<2, f32>) -> bool {
		self.base.inside(position)
	}
	fn resize(&mut self, state: &(impl render::Has<render::State> + render::Has<render::UIState>), mut rect: Rect) {
		rect.max[Y] = rect.min[Y] + self.scale * (rect.max[X] - rect.min[X]);
		self.base.resize(state, rect)
	}
}
