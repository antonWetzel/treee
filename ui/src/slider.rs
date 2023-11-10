use math::{X, Y};

use crate::{length, Anchor, Area, Element, State};

pub struct Slider<Background: Element, Marker: Element> {
	background: Background,
	marker: Area<Marker>,
	event: fn(f32) -> Background::Event,
	active: bool,
}

impl<Background: Element, Marker: Element> Slider<Background, Marker> {
	pub fn new(background: Background, marker: Marker, event: fn(f32) -> Background::Event) -> Self {
		Self {
			background,
			marker: Area::new(
				marker,
				Anchor::square([length!(h 0.25), length!(h 0.25)].into(), length!(h 0.5)),
			),
			event,
			active: false,
		}
	}
}

impl<Background: Element, Marker: Element> render::UIElement for Slider<Background, Marker> {
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.background.render(ui_pass);
		self.marker.render(ui_pass);
	}

	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		self.background.collect(collector);
		self.marker.collect(collector);
	}
}

impl<Background: Element, Marker: Element> Element for Slider<Background, Marker> {
	type Event = Background::Event;

	fn inside(&self, position: math::Vector<2, f32>) -> bool {
		self.active || self.background.inside(position)
	}

	fn bounding_rect(&self) -> crate::Rect {
		self.background.bounding_rect()
	}

	fn resize(&mut self, state: &impl State, rect: crate::Rect) {
		self.background.resize(state, rect);
		self.marker.resize(state, self.background.bounding_rect());
	}

	fn click(&mut self, _state: &impl State, _position: math::Vector<2, f32>) -> Option<Self::Event> {
		self.active = true;
		None
	}

	fn hover(&mut self, state: &impl State, position: math::Vector<2, f32>, pressed: bool) -> Option<Self::Event> {
		if !pressed {
			self.active = false;
			return None;
		}

		let rect = self.background.bounding_rect();
		let size = rect.size();
		let start = rect.min[X] + size[Y] * 0.5;

		let percent = (position[X] - start) / (size[X] - size[Y]);
		let percent = percent.clamp(0.0, 1.0);

		self.marker.anchor = Anchor::square(
			[length!(w percent, h 0.25 - percent), length!(h 0.25)].into(),
			length!(h 0.5),
		);
		self.marker.resize(state, self.background.bounding_rect());
		Some((self.event)(percent))
	}
}

pub struct DoubleSlider<Background: Element, Marker: Element> {
	background: Background,
	lower: Area<Marker>,
	upper: Area<Marker>,
	event: fn(f32, f32) -> Background::Event,
	active: bool,
}

impl<Background: Element, Marker: Element> DoubleSlider<Background, Marker> {
	pub fn new(background: Background, lower: Marker, upper: Marker, event: fn(f32, f32) -> Background::Event) -> Self {
		Self {
			background,
			lower: Area::new(
				lower,
				Anchor::square([length!(h 0.25), length!(h 0.25)].into(), length!(h 0.5)),
			),
			upper: Area::new(
				upper,
				Anchor::square(
					[length!(w 1.0, h -0.75), length!(h 0.25)].into(),
					length!(h 0.5),
				),
			),
			event,
			active: false,
		}
	}
}

impl<Background: Element, Marker: Element> render::UIElement for DoubleSlider<Background, Marker> {
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.background.render(ui_pass);
		self.lower.render(ui_pass);
		self.upper.render(ui_pass);
	}

	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		self.background.collect(collector);
		self.lower.collect(collector);
		self.upper.collect(collector);
	}
}

impl<Background: Element, Marker: Element> Element for DoubleSlider<Background, Marker> {
	type Event = Background::Event;

	fn inside(&self, position: math::Vector<2, f32>) -> bool {
		self.active || self.background.inside(position)
	}

	fn bounding_rect(&self) -> crate::Rect {
		self.background.bounding_rect()
	}

	fn resize(&mut self, state: &impl State, rect: crate::Rect) {
		self.background.resize(state, rect);
		self.lower.resize(state, self.background.bounding_rect());
		self.upper.resize(state, self.background.bounding_rect());
	}

	fn click(&mut self, _state: &impl State, _position: math::Vector<2, f32>) -> Option<Self::Event> {
		self.active = true;
		None
	}

	fn hover(&mut self, state: &impl State, position: math::Vector<2, f32>, pressed: bool) -> Option<Self::Event> {
		if !pressed {
			self.active = false;
			return None;
		}

		let rect = self.background.bounding_rect();
		let size = rect.size();
		let start = rect.min[X] + size[Y] * 0.25;

		let percent = (position[X] - (start + size[Y] * 0.25)) / (size[X] - size[Y]);
		let percent = percent.clamp(0.0, 1.0);

		let [mut current_lower, mut current_upper] = [&self.lower, &self.upper].map(|v| {
			let rect = v.bounding_rect();
			(rect.min[X] - start) / (size[X] - size[Y])
		});

		let marker = if (current_lower - percent).abs() < (current_upper - percent).abs() {
			current_lower = percent;
			&mut self.lower
		} else {
			current_upper = percent;
			&mut self.upper
		};

		marker.anchor = Anchor::square(
			[length!(w percent, h 0.25 - percent), length!(h 0.25)].into(),
			length!(h 0.5),
		);
		marker.resize(state, self.background.bounding_rect());
		Some((self.event)(current_lower, current_upper))
	}
}
