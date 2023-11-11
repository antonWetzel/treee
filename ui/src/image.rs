use math::Vector;

use crate::{Element, Rect, State};

pub struct Image<Event> {
	image: render::UIImage,
	rect: Rect,
	event: std::marker::PhantomData<Event>,
}

impl<Event> Image<Event> {
	pub fn new(state: &impl State, texture: &render::Texture) -> Self {
		let position = [0.0, 0.0].into();
		let size = [16.0, 16.0].into();
		Self {
			image: render::UIImage::new(state, position, size, texture),
			rect: Rect { min: position, max: position + size },
			event: std::marker::PhantomData,
		}
	}
}

impl<E> render::UIElement for Image<E> {
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.image.render(ui_pass);
	}
	fn collect<'a>(&'a self, _collector: &mut render::UICollector<'a>) {}
}

impl<E> Element for Image<E> {
	type Event = E;

	fn inside(&self, position: Vector<2, f32>) -> bool {
		self.rect.inside(position)
	}

	fn bounding_rect(&self) -> Rect {
		self.rect
	}

	fn click(&mut self, _state: &impl State, _position: Vector<2, f32>) -> Option<Self::Event> {
		None
	}
	fn release(&mut self, _position: Vector<2, f32>) -> bool {
		false
	}

	fn hover(&mut self, _state: &impl State, _position: Vector<2, f32>, _pressed: bool) -> Option<Self::Event> {
		None
	}

	fn resize(&mut self, state: &impl State, rect: Rect) {
		self.rect = rect;
		self.image.update(state, rect.min, rect.size());
	}
}
