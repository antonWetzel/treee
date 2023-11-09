use std::ops::Not;

use math::Vector;
use render::Has;

use crate::{Element, Rect};

pub struct Popup<Base: Element, Popup: Element>
where
	Base::Event: From<Popup::Event>,
{
	base: Base,
	popup: Popup,
	active: bool,
	event: fn() -> Base::Event,
}

impl<Base: Element, P: Element> Popup<Base, P>
where
	Base::Event: From<P::Event>,
{
	pub fn new(base: Base, popup: P, event: fn() -> Base::Event) -> Self {
		Self { base, popup, active: false, event }
	}
}

impl<Base: Element, P: Element> render::UIElement for Popup<Base, P>
where
	Base::Event: From<P::Event>,
{
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.base.render(ui_pass);
		if self.active {
			self.popup.render(ui_pass);
		}
	}

	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		self.base.collect(collector);
		if self.active {
			self.popup.collect(collector);
		}
	}
}

impl<Base: Element, P: Element> Element for Popup<Base, P>
where
	Base::Event: From<P::Event>,
{
	type Event = Base::Event;

	fn inside(&self, position: Vector<2, f32>) -> bool {
		self.base.inside(position) || self.active && self.popup.inside(position)
	}

	fn bounding_rect(&self) -> Rect {
		self.base.bounding_rect().merge(self.popup.bounding_rect())
	}

	fn resize(&mut self, state: &(impl Has<render::State> + Has<render::UIState>), rect: Rect) {
		self.base.resize(state, rect);
		self.popup.resize(state, self.base.bounding_rect());
	}

	fn click(&mut self, position: Vector<2, f32>) -> Option<Self::Event> {
		if self.base.inside(position) {
			return self.base.click(position);
		}
		if self.active && self.popup.inside(position) {
			return self.popup.click(position).map(|e| e.into());
		}
		None
	}

	fn hover(&mut self, position: Vector<2, f32>) -> Option<Self::Event> {
		if self.active != self.inside(position) {
			self.active = self.active.not();
			Some((self.event)())
		} else {
			None
		}
	}
}
