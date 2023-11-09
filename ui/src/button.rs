use math::Vector;
use render::Has;

use crate::{Element, Rect};

pub struct Button<Base: Element> {
	element: Base,
	event: fn() -> Base::Event,
}

impl<Base: Element> Button<Base> {
	pub fn new(element: Base, event: fn() -> Base::Event) -> Self {
		Self { element, event }
	}
}

impl<Base: Element> render::UIElement for Button<Base> {
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.element.render(ui_pass)
	}

	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		self.element.collect(collector)
	}
}

impl<Base: Element> Element for Button<Base> {
	type Event = Base::Event;

	fn inside(&self, position: Vector<2, f32>) -> bool {
		self.element.inside(position)
	}

	fn click(&mut self, _position: Vector<2, f32>) -> Option<Self::Event> {
		Some((self.event)())
	}

	fn resize(&mut self, state: &(impl Has<render::State> + Has<render::UIState>), rect: Rect) {
		self.element.resize(state, rect)
	}
}
