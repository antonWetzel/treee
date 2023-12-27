use std::ops::Not;

use math::{ Vector, X, Y };

use crate::{ Anchor, Element, Rect, State };


/// todo: split into composable types
pub struct Area<Base: Element> {
	pub anchor: Anchor,
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


	fn bounding_rect(&self) -> Rect {
		self.base.bounding_rect()
	}


	fn click(&mut self, state: &impl State, position: Vector<2, f32>) -> Option<Self::Event> {
		if self.inside(position).not() {
			return None;
		}
		self.base.click(state, position)
	}


	fn release(&mut self, position: Vector<2, f32>) -> bool {
		self.base.release(position)
	}


	fn hover(&mut self, state: &impl State, position: Vector<2, f32>, pressed: bool) -> Option<Self::Event> {
		self.base.hover(state, position, pressed)
	}


	fn resize(&mut self, state: &impl State, rect: Rect) {
		let size = rect.size();
		self.base.resize(
			state,
			Rect {
				min: [
					self.anchor.min[X].map(rect.min[X], size),
					self.anchor.min[Y].map(rect.min[Y], size),
				]
					.into(),
				max: [
					self.anchor.max[X].map(rect.min[X], size),
					self.anchor.max[Y].map(rect.min[Y], size),
				]
					.into(),
			},
		)
	}
}


impl<Base: Element> std::ops::Deref for Area<Base> {
	type Target = Base;


	fn deref(&self) -> &Self::Target {
		&self.base
	}
}


impl<Base: Element> std::ops::DerefMut for Area<Base> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.base
	}
}
