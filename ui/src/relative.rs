use math::{ X, Y };

use crate::{ Element, Horizontal, Rect, State, Vertical };


pub trait RelativeDirection {
	fn rect(rect: Rect, scale: f32) -> Rect;
}


impl RelativeDirection for Horizontal {
	fn rect(mut rect: Rect, scale: f32) -> Rect {
		rect.max[Y] = rect.min[Y] + scale * (rect.max[X] - rect.min[X]);
		rect
	}
}


impl RelativeDirection for Vertical {
	fn rect(mut rect: Rect, scale: f32) -> Rect {
		rect.max[X] = rect.min[X] + scale * (rect.max[Y] - rect.min[Y]);
		rect
	}
}


pub struct Relative<D: RelativeDirection, Base: Element> {
	base: Base,
	scale: f32,
	phantom: std::marker::PhantomData<D>,
}


impl<D: RelativeDirection, Base: Element> Relative<D, Base> {
	pub fn new(base: Base, scale: f32) -> Self {
		Self {
			base,
			scale,
			phantom: std::marker::PhantomData,
		}
	}


	pub fn square(base: Base) -> Self {
		Self::new(base, 1.0)
	}
}


impl<D: RelativeDirection, Base: Element> render::UIElement for Relative<D, Base> {
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.base.render(ui_pass);
	}


	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		self.base.collect(collector);
	}
}


impl<D: RelativeDirection, Base: Element> Element for Relative<D, Base> {
	type Event = Base::Event;


	fn bounding_rect(&self) -> crate::Rect {
		self.base.bounding_rect()
	}


	fn click(&mut self, state: &impl State, position: math::Vector<2, f32>) -> Option<Self::Event> {
		if self.inside(position) {
			return self.base.click(state, position);
		}
		None
	}


	fn release(&mut self, position: math::Vector<2, f32>) -> bool {
		self.base.release(position)
	}


	fn hover(&mut self, state: &impl State, position: math::Vector<2, f32>, pressed: bool) -> Option<Self::Event> {
		self.base.hover(state, position, pressed)
	}


	fn inside(&self, position: math::Vector<2, f32>) -> bool {
		self.base.inside(position)
	}


	fn resize(&mut self, state: &(impl render::Has<render::State> + render::Has<render::UIState>), rect: Rect) {
		self.base.resize(state, D::rect(rect, self.scale))
	}
}


impl<D: RelativeDirection, Base: Element> std::ops::Deref for Relative<D, Base> {
	type Target = Base;


	fn deref(&self) -> &Self::Target {
		&self.base
	}
}


impl<D: RelativeDirection, Base: Element> std::ops::DerefMut for Relative<D, Base> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.base
	}
}
