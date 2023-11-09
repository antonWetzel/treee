use std::ops::Not;

use math::{Vector, X, Y};
use render::Has;

use crate::{Anchor, Element, Rect};

pub struct Area<Base: Element> {
	anchor: Anchor,
	base: Base,
}

impl<Base: Element> Area<Base> {
	pub fn new(base: Base, anchor: Anchor) -> Self {
		Self { base, anchor }
	}
}

impl<Base: Element> render::UIElement for Area<Base> {
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.base.render(ui_pass)
	}

	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		self.base.collect(collector)
	}
}

impl<Base: Element> Element for Area<Base> {
	type Event = Base::Event;

	fn inside(&self, position: Vector<2, f32>) -> bool {
		self.base.inside(position)
	}

	fn click(&mut self, position: Vector<2, f32>) -> Option<Self::Event> {
		if self.inside(position).not() {
			return None;
		}
		self.base.click(position)
	}

	fn resize(&mut self, state: &(impl Has<render::State> + Has<render::UIState>), rect: Rect) {
		self.base.resize(
			state,
			Rect {
				position: [
					self.anchor.position[X].map(rect.position[X], rect.size),
					self.anchor.position[Y].map(rect.position[Y], rect.size),
				]
				.into(),
				size: [
					self.anchor.size[X].map(rect.position[X], rect.size),
					self.anchor.size[Y].map(rect.position[Y], rect.size),
				]
				.into(),
			},
		)
	}
}
