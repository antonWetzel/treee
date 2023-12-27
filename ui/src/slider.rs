use std::ops::Not;

use math::{ Dimension, Vector, X, Y };

use crate::{ length, Anchor, Area, Element, Horizontal, Length, State, Vertical };


pub trait SliderDirection {
	const AXIS_0: Dimension;
	const AXIS_1: Dimension;


	fn length(v_0: f32, v_1: f32) -> Length;


	fn combine(v_0: Length, v_1: Length) -> Vector<2, Length>;
}


impl SliderDirection for Horizontal {
	const AXIS_0: Dimension = X;
	const AXIS_1: Dimension = Y;


	fn length(v_0: f32, v_1: f32) -> Length {
		length!(w v_0, h v_1)
	}


	fn combine(v_0: Length, v_1: Length) -> Vector<2, Length> {
		[v_0, v_1].into()
	}
}


impl SliderDirection for Vertical {
	const AXIS_0: Dimension = Y;
	const AXIS_1: Dimension = X;


	fn length(v_0: f32, v_1: f32) -> Length {
		length!(w v_1, h v_0)
	}


	fn combine(v_0: Length, v_1: Length) -> Vector<2, Length> {
		[v_1, v_0].into()
	}
}


pub struct Slider<D: SliderDirection, Background: Element, Marker: Element> {
	background: Background,
	marker: Area<Marker>,
	event: fn(f32) -> Background::Event,
	active: bool,
	phantom: std::marker::PhantomData<D>,
}


impl<D: SliderDirection, Background: Element, Marker: Element> Slider<D, Background, Marker> {
	pub fn new(background: Background, marker: Marker, default: f32, event: fn(f32) -> Background::Event) -> Self {
		Self {
			background,
			marker: Area::new(
				marker,
				Anchor::square(
					D::combine(D::length(default, 0.25 - default), D::length(0.0, 0.25)),
					D::length(0.0, 0.5),
				),
			),
			event,
			active: false,
			phantom: std::marker::PhantomData,
		}
	}


	pub fn set_marker(&mut self, state: &impl State, percent: f32) {
		self.marker.anchor = Anchor::square(
			D::combine(D::length(percent, 0.25 - percent), D::length(0.0, 0.25)),
			D::length(0.0, 0.5),
		);
		self.marker.resize(state, self.background.bounding_rect());
	}
}


impl<D: SliderDirection, Background: Element, Marker: Element> render::UIElement for Slider<D, Background, Marker> {
	fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
		self.background.render(ui_pass);
		self.marker.render(ui_pass);
	}


	fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
		self.background.collect(collector);
		self.marker.collect(collector);
	}
}


impl<D: SliderDirection, Background: Element, Marker: Element> Element for Slider<D, Background, Marker> {
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


	fn release(&mut self, _position: math::Vector<2, f32>) -> bool {
		if self.active.not() {
			return false;
		}
		self.active = false;
		true
	}


	fn hover(&mut self, state: &impl State, position: math::Vector<2, f32>, pressed: bool) -> Option<Self::Event> {
		if !pressed {
			self.active = false;
			return None;
		}

		let rect = self.background.bounding_rect();
		let size = rect.size();
		let start = rect.min[D::AXIS_0] + size[D::AXIS_1] * 0.5;

		let percent = (position[D::AXIS_0] - start) / (size[D::AXIS_0] - size[D::AXIS_1]);
		let percent = percent.clamp(0.0, 1.0);

		self.set_marker(state, percent);
		Some((self.event)(percent))
	}
}


pub struct DoubleSlider<D: SliderDirection, Background: Element, Marker: Element> {
	background: Background,
	lower: Area<Marker>,
	upper: Area<Marker>,
	event: fn(f32, f32) -> Background::Event,
	active: bool,
	phantom: std::marker::PhantomData<D>,
}


impl<D: SliderDirection, Background: Element, Marker: Element> DoubleSlider<D, Background, Marker> {
	pub fn new(background: Background, lower: Marker, upper: Marker, event: fn(f32, f32) -> Background::Event) -> Self {
		Self {
			background,
			lower: Area::new(
				lower,
				Anchor::square(
					D::combine(D::length(0.0, 0.25), D::length(0.0, 0.25)),
					D::length(0.0, 0.5),
				),
			),
			upper: Area::new(
				upper,
				Anchor::square(
					D::combine(D::length(1.0, -0.75), D::length(0.0, 0.25)),
					D::length(0.0, 0.5),
				),
			),
			event,
			active: false,
			phantom: std::marker::PhantomData,
		}
	}
}


impl<D: SliderDirection, Background: Element, Marker: Element> render::UIElement
	for DoubleSlider<D, Background, Marker>
{
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


impl<D: SliderDirection, Background: Element, Marker: Element> Element for DoubleSlider<D, Background, Marker> {
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


	fn release(&mut self, _position: math::Vector<2, f32>) -> bool {
		if self.active.not() {
			return false;
		}
		self.active = false;
		true
	}


	fn hover(&mut self, state: &impl State, position: math::Vector<2, f32>, pressed: bool) -> Option<Self::Event> {
		if !pressed {
			self.active = false;
			return None;
		}

		let rect = self.background.bounding_rect();
		let size = rect.size();
		let start = rect.min[D::AXIS_0] + size[D::AXIS_1] * 0.25;

		let percent = (position[D::AXIS_0] - (start + size[D::AXIS_1] * 0.25)) / (size[D::AXIS_0] - size[D::AXIS_1]);
		let percent = percent.clamp(0.0, 1.0);

		let [mut current_lower, mut current_upper] = [&self.lower, &self.upper].map(|v| {
			let rect = v.bounding_rect();
			(rect.min[D::AXIS_0] - start) / (size[D::AXIS_0] - size[D::AXIS_1])
		});

		let marker = if (current_lower - percent).abs() < (current_upper - percent).abs() {
			current_lower = percent;
			&mut self.lower
		} else {
			current_upper = percent;
			&mut self.upper
		};

		marker.anchor = Anchor::square(
			D::combine(D::length(percent, 0.25 - percent), D::length(0.0, 0.25)),
			D::length(0.0, 0.5),
		);
		marker.resize(state, self.background.bounding_rect());
		Some((self.event)(current_lower, current_upper))
	}
}
