use math::{X, Y};

use crate::{Element, Rect};

pub trait Direction {
	fn first(rect: Rect) -> Rect;
	fn second(rect: Rect) -> Rect;
}

pub struct Horizontal;
impl Direction for Horizontal {
	fn first(mut rect: Rect) -> Rect {
		rect.max[Y] = (rect.min[Y] + rect.max[Y]) / 2.0;
		rect
	}

	fn second(mut rect: Rect) -> Rect {
		rect.min[Y] = (rect.min[Y] + rect.max[Y]) / 2.0;
		rect
	}
}

pub struct Vertical;
impl Direction for Vertical {
	fn first(mut rect: Rect) -> Rect {
		rect.max[X] = (rect.min[X] + rect.max[X]) / 2.0;
		rect
	}

	fn second(mut rect: Rect) -> Rect {
		rect.min[X] = (rect.min[X] + rect.max[X]) / 2.0;
		rect
	}
}

pub struct Split<D: Direction, First: Element, Second: Element>
where
	First::Event: From<Second::Event>,
{
	first: First,
	second: Second,
	phantom: std::marker::PhantomData<D>,
}

impl<D: Direction, First: Element, Second: Element> Split<D, First, Second>
where
	First::Event: From<Second::Event>,
{
	pub fn new(left: First, right: Second) -> Self {
		Self {
			first: left,
			second: right,
			phantom: std::marker::PhantomData,
		}
	}
}

impl<D: Direction, First: Element, Second: Element> render::UIElement for Split<D, First, Second>
where
	First::Event: From<Second::Event>,
{
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.first.render(ui_pass);
		self.second.render(ui_pass);
	}
}

impl<D: Direction, First: Element, Second: Element> Element for Split<D, First, Second>
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

	fn resize(&mut self, state: &(impl render::Has<render::State> + render::Has<render::UIState>), rect: crate::Rect) {
		self.first.resize(state, D::first(rect));
		self.second.resize(state, D::second(rect));
	}

	fn hover(&mut self, position: math::Vector<2, f32>) -> Option<Self::Event> {
		if self.first.inside(position) {
			return self.first.hover(position);
		}
		if self.second.inside(position) {
			return self.second.hover(position).map(|v| v.into());
		}
		None
	}

	fn click(&mut self, position: math::Vector<2, f32>) -> Option<Self::Event> {
		if self.first.inside(position) {
			return self.first.click(position);
		}
		if self.second.inside(position) {
			return self.second.click(position).map(|v| v.into());
		}
		None
	}
}
