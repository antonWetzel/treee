use math::{X, Y};

use crate::{Element, Horizontal, Rect, State, Vertical};

pub trait SplitDirection {
	fn first(rect: Rect, sep: f32) -> Rect;
	fn second(rect: Rect, sep: f32) -> Rect;
}

impl SplitDirection for Horizontal {
	fn first(mut rect: Rect, sep: f32) -> Rect {
		rect.max[Y] = rect.min[Y] + (rect.max[Y] - rect.min[Y]) * sep;
		rect
	}

	fn second(mut rect: Rect, sep: f32) -> Rect {
		rect.min[Y] += (rect.max[Y] - rect.min[Y]) * sep;
		rect
	}
}

impl SplitDirection for Vertical {
	fn first(mut rect: Rect, sep: f32) -> Rect {
		rect.max[X] = rect.min[X] + (rect.max[X] - rect.min[X]) * sep;
		rect
	}

	fn second(mut rect: Rect, sep: f32) -> Rect {
		rect.min[X] += (rect.max[X] - rect.min[X]) * sep;
		rect
	}
}

pub struct Split<D: SplitDirection, First: Element, Second: Element>
where
	First::Event: From<Second::Event>,
{
	pub first: First,
	pub second: Second,
	sep: f32,
	phantom: std::marker::PhantomData<D>,
}

impl<D: SplitDirection, First: Element, Second: Element> Split<D, First, Second>
where
	First::Event: From<Second::Event>,
{
	pub fn new(left: First, right: Second, sep: f32) -> Self {
		Self {
			first: left,
			second: right,
			sep,
			phantom: std::marker::PhantomData,
		}
	}
}

impl<D: SplitDirection, First: Element, Second: Element> render::UIElement for Split<D, First, Second>
where
	First::Event: From<Second::Event>,
{
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.first.render(ui_pass);
		self.second.render(ui_pass);
	}

	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		self.first.collect(collector);
		self.second.collect(collector);
	}
}

impl<D: SplitDirection, First: Element, Second: Element> Element for Split<D, First, Second>
where
	First::Event: From<Second::Event>,
{
	type Event = First::Event;

	fn inside(&self, position: math::Vector<2, f32>) -> bool {
		self.first.inside(position) || self.second.inside(position)
	}

	fn bounding_rect(&self) -> Rect {
		self.first
			.bounding_rect()
			.merge(self.second.bounding_rect())
	}

	fn resize(&mut self, state: &impl State, rect: crate::Rect) {
		self.first.resize(state, D::first(rect, self.sep));
		self.second.resize(state, D::second(rect, self.sep));
	}

	fn hover(&mut self, state: &impl State, position: math::Vector<2, f32>, pressed: bool) -> Option<Self::Event> {
		if self.first.inside(position) {
			return self.first.hover(state, position, pressed);
		}
		if self.second.inside(position) {
			return self
				.second
				.hover(state, position, pressed)
				.map(|v| v.into());
		}
		None
	}

	fn click(&mut self, state: &impl State, position: math::Vector<2, f32>) -> Option<Self::Event> {
		if self.first.inside(position) {
			return self.first.click(state, position);
		}
		if self.second.inside(position) {
			return self.second.click(state, position).map(|v| v.into());
		}
		None
	}
	fn release(&mut self, position: math::Vector<2, f32>) -> bool {
		self.first.release(position) | self.second.release(position)
	}
}
