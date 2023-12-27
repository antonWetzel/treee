use std::ops::Not;

use math::Vector;

use crate::{ Element, Rect, State };


pub struct Popup<Base: Element, Popup: Element>
where
	Base::Event: From<Popup::Event>,
{
	pub base: Base,
	pub popup: Popup,
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
		self.active || self.base.inside(position)
	}


	fn bounding_rect(&self) -> Rect {
		self.base.bounding_rect()
	}


	fn resize(&mut self, state: &impl State, rect: Rect) {
		self.base.resize(state, rect);
		self.popup.resize(state, self.base.bounding_rect());
	}


	fn click(&mut self, state: &impl State, position: Vector<2, f32>) -> Option<Self::Event> {
		if self.base.inside(position) {
			return self.base.click(state, position);
		}
		if self.active && self.popup.inside(position) {
			return self.popup.click(state, position).map(|e| e.into());
		}
		None
	}


	fn release(&mut self, position: Vector<2, f32>) -> bool {
		if self.active.not() {
			return false;
		}
		if self.base.inside(position) || self.popup.inside(position) {
			return false;
		}
		self.active = false;
		true
	}


	fn hover(&mut self, state: &impl State, position: Vector<2, f32>, pressed: bool) -> Option<Self::Event> {
		if self.base.inside(position) {
			if self.active {
				return None;
			}
			if pressed.not() {
				self.active = true;
				return Some((self.event)());
			}
			return None;
		}

		if self.active {
			if self.popup.inside(position) {
				return self.popup.hover(state, position, pressed).map(|v| v.into());
			} else {
				self.active = false;
				return Some((self.event)());
			}
		}
		None
	}
}
