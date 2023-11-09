use math::Vector;
use render::Has;

use crate::{Element, Event, Rect};

pub struct Image<Event> {
	image: render::UIImage,
	rect: Rect,
	event: std::marker::PhantomData<Event>,
}

impl<Event> Image<Event> {
	pub fn new(state: &(impl Has<render::State> + Has<render::UIState>), texture: &render::Texture) -> Self {
		let position = [0.0, 0.0].into();
		let size = [16.0, 16.0].into();
		Self {
			image: render::UIImage::new(state, position, size, texture),
			rect: Rect { position, size },
			event: std::marker::PhantomData,
		}
	}
}

impl<E> render::UIElement for Image<E> {
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.image.render(ui_pass);
	}
}

impl<E> Element for Image<E>
where
	E: Event,
{
	type Event = E;

	fn inside(&self, position: Vector<2, f32>) -> bool {
		self.rect.inside(position)
	}

	fn resize(&mut self, state: &(impl Has<render::State> + Has<render::UIState>), rect: Rect) {
		self.rect = rect;
		self.image.update(state, rect.position, rect.size);
	}
}
