use math::Vector;

use crate::{ Element, Rect, State };


pub struct Button<Base: Element> {
	base: Base,
	event: fn() -> Base::Event,
}


impl<Base: Element> Button<Base> {
	pub fn new(element: Base, event: fn() -> Base::Event) -> Self {
		Self { base: element, event }
	}
}


impl<Base: Element> render::UIElement for Button<Base> {
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.base.render(ui_pass)
	}


	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		self.base.collect(collector)
	}
}


impl<Base: Element> Element for Button<Base> {
	type Event = Base::Event;


	fn inside(&self, position: Vector<2, f32>) -> bool {
		self.base.inside(position)
	}


	fn bounding_rect(&self) -> Rect {
		self.base.bounding_rect()
	}


	fn click(&mut self, _state: &impl State, _position: Vector<2, f32>) -> Option<Self::Event> {
		Some((self.event)())
	}


	fn release(&mut self, position: Vector<2, f32>) -> bool {
		self.base.release(position)
	}


	fn hover(&mut self, state: &impl State, position: Vector<2, f32>, pressed: bool) -> Option<Self::Event> {
		self.base.hover(state, position, pressed)
	}


	fn resize(&mut self, state: &impl State, rect: Rect) {
		self.base.resize(state, rect)
	}
}


impl<Base: Element> std::ops::Deref for Button<Base> {
	type Target = Base;


	fn deref(&self) -> &Self::Target {
		&self.base
	}
}


impl<Base: Element> std::ops::DerefMut for Button<Base> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.base
	}
}
