use math::Vector;

use crate::{Element, Rect, State};

pub struct Hide<Base: Element> {
	base: Base,
	pub active: bool,
}

impl<Base: Element> Hide<Base> {
	pub fn new(base: Base, active: bool) -> Self {
		Self { base, active }
	}
}

impl<Base: Element> render::UIElement for Hide<Base> {
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		if self.active {
			self.base.render(ui_pass)
		}
	}

	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		if self.active {
			self.base.collect(collector)
		}
	}
}

impl<Base: Element> Element for Hide<Base> {
	type Event = Base::Event;

	fn inside(&self, position: Vector<2, f32>) -> bool {
		self.active && self.base.inside(position)
	}

	fn bounding_rect(&self) -> Rect {
		// todo: none rect?
		self.base.bounding_rect()
	}

	fn click(&mut self, state: &impl State, position: Vector<2, f32>) -> Option<Self::Event> {
		if self.active {
			return None;
		}
		self.base.click(state, position)
	}
	fn release(&mut self, position: Vector<2, f32>) -> bool {
		self.base.release(position)
	}

	fn hover(&mut self, state: &impl State, position: Vector<2, f32>, pressed: bool) -> Option<Self::Event> {
		if self.active {
			return None;
		}
		self.base.hover(state, position, pressed)
	}

	fn resize(&mut self, state: &impl State, rect: Rect) {
		self.base.resize(state, rect)
	}
}

impl<Base: Element> std::ops::Deref for Hide<Base> {
	type Target = Base;
	fn deref(&self) -> &Self::Target {
		&self.base
	}
}

impl<Base: Element> std::ops::DerefMut for Hide<Base> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.base
	}
}
