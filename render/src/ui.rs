use std::sync::Arc;

use glyphon::{
	fontdb::Source, Attrs, Buffer, Color, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea,
	TextAtlas, TextBounds, TextRenderer,
};
use wgpu::SurfaceConfiguration;

use crate::{Has, RenderPass, State};

pub struct UI {
	font_system: FontSystem,
	renderer: TextRenderer,
	cashe: SwashCache,
	atlas: TextAtlas,
}

impl UI {
	pub fn new(state: &impl Has<State>) -> Self {
		let state = state.get();
		let font_system = FontSystem::new_with_fonts(
			[Source::Binary(Arc::new(include_bytes!(
				"../assets/Inter-Bold.ttf"
			)))]
			.into_iter(),
		);
		let cashe = SwashCache::new();
		let mut atlas = TextAtlas::new(&state.device, &state.queue, state.surface_format);
		let renderer = TextRenderer::new(
			&mut atlas,
			&state.device,
			wgpu::MultisampleState::default(),
			None,
		);
		Self { font_system, renderer, cashe, atlas }
	}

	pub fn render<'a>(&'a self, pass: &mut RenderPass<'a>) {
		self.renderer.render(&self.atlas, pass).unwrap();
	}
}

pub struct UIElement {
	buffer: Buffer,
}

impl UIElement {
	pub fn new<'a>(ui: &mut UI, text: &str) -> Self {
		let mut buffer = Buffer::new(&mut ui.font_system, Metrics::new(30.0, 42.0));
		buffer.set_size(&mut ui.font_system, 300.0, 300.0);
		buffer.set_text(
			&mut ui.font_system,
			text,
			Attrs::new().family(Family::Name("Inter-Bold")),
			Shaping::Advanced,
		);
		Self { buffer }
	}

	pub fn prepare(&self, ui: &mut UI, state: &impl Has<State>, config: &SurfaceConfiguration) {
		let state = state.get();
		ui.renderer
			.prepare(
				&state.device,
				&state.queue,
				&mut ui.font_system,
				&mut ui.atlas,
				Resolution {
					width: config.width,
					height: config.height,
				},
				[TextArea {
					buffer: &self.buffer,
					left: 500.0,
					top: 100.0,
					scale: 1.0,
					bounds: TextBounds::default(),
					default_color: Color::rgb(0, 0, 0),
				}],
				&mut ui.cashe,
			)
			.unwrap();
	}
}
