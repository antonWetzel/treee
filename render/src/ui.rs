use math::{Vector, X, Y};
use wgpu::SurfaceConfiguration;
use wgpu_text::{
	glyph_brush::{ab_glyph::FontRef, Section, Text},
	BrushBuilder, TextBrush,
};

use crate::{Has, RenderPass, State};

pub struct UI {
	brush: TextBrush<FontRef<'static>>,
}

impl UI {
	pub fn new(state: &impl Has<State>, config: &SurfaceConfiguration) -> Self {
		let state = state.get();
		let font = include_bytes!("ui_Urbanist-Bold.ttf");
		let font = FontRef::try_from_slice(font).unwrap();

		let brush = BrushBuilder::using_font(font).build(&state.device, config.width, config.height, config.format);
		Self { brush }
	}

	pub fn resize(&mut self, state: &impl Has<State>, config: &SurfaceConfiguration) {
		self.brush.resize_view(
			config.width as f32,
			config.height as f32,
			&state.get().queue,
		)
	}

	pub fn queue(&mut self, state: &impl Has<State>, target: &impl UICollect) {
		let state = state.get();
		let mut collector = UICollector { data: Vec::new() };
		target.collect(&mut collector);
		self.brush
			.queue(&state.device, &state.queue, collector.data)
			.unwrap();
	}

	pub fn render<'a>(&'a self, mut render_pass: RenderPass<'a>) -> RenderPass<'a> {
		self.brush.draw(&mut render_pass);
		render_pass
	}
}

pub trait UICollect {
	fn collect<'a>(&'a self, collector: &mut UICollector<'a>);
}

pub struct UICollector<'a> {
	data: Vec<Section<'a>>,
}

impl<'a> UICollector<'a> {
	pub fn add_element(&mut self, element: &'a UIElement) {
		self.data.push(Section {
			screen_position: (element.position[X], element.position[Y]),
			text: element
				.text
				.iter()
				.map(|t| {
					Text::new(t.as_str())
						.with_scale(element.font_size)
						.with_color([0.0, 0.0, 0.0, 1.0])
				})
				.collect(),
			..Default::default()
		});
	}
}

pub struct UIElement {
	pub position: Vector<2, f32>,
	pub text: Vec<String>,
	pub font_size: f32,
}

impl UIElement {
	pub fn new(text: Vec<String>, position: Vector<2, f32>, font_size: f32) -> Self {
		Self { text, position, font_size }
	}
}
