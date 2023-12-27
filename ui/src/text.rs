use math::{ Vector, X, Y };

use crate::{ Element, Rect, State };


pub struct Text<Event> {
	text: render::UIText,
	rect: Rect,
	event: std::marker::PhantomData<Event>,
}


pub type HorizontalAlign = render::UIHorizontalAlign;
pub type VerticalAlign = render::UIVerticalAlign;


impl<Event> Text<Event> {
	pub fn new(text: Vec<String>, horizontal: HorizontalAlign, vertical: VerticalAlign) -> Self {
		let position = [0.0, 0.0].into();
		let size = 30.0;
		Self {
			text: render::UIText::new(text, position, size, horizontal, vertical),
			rect: Rect {
				min: position,
				max: position + [16.0, 16.0].into(),
			},
			event: std::marker::PhantomData,
		}
	}


	pub fn update_text(&mut self, text: Vec<String>) {
		self.text.text = text;
	}
}


impl<E> render::UIElement for Text<E> {
	fn render<'a>(&'a self, _ui_pass: &mut render::UIPass<'a>) { }


	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		self.text.collect(collector);
	}
}


impl<E> Element for Text<E> {
	type Event = E;


	fn inside(&self, position: Vector<2, f32>) -> bool {
		self.rect.inside(position)
	}


	fn bounding_rect(&self) -> Rect {
		self.rect
	}


	fn click(&mut self, _state: &impl State, _position: Vector<2, f32>) -> Option<Self::Event> { None }


	fn release(&mut self, _position: Vector<2, f32>) -> bool {
		false
	}


	fn hover(&mut self, _state: &impl State, _position: Vector<2, f32>, _pressed: bool) -> Option<Self::Event> { None }


	fn resize(&mut self, _state: &impl State, rect: Rect) {
		self.rect = rect;
		self.text.position[X] = match self.text.horizontal {
			HorizontalAlign::Left => rect.min[X],
			HorizontalAlign::Center => (rect.min[X] + rect.max[X]) / 2.0,
			HorizontalAlign::Right => rect.max[X],
		};

		self.text.position[Y] = match self.text.vertical {
			VerticalAlign::Top => rect.min[Y],
			VerticalAlign::Center => (rect.min[Y] + rect.max[Y]) / 2.0,
			VerticalAlign::Bottom => rect.max[Y],
		};
	}
}
