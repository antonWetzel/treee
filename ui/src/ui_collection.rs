#[macro_export]
macro_rules! UICollection {
	(
		type Event = $event:ident;

		$visibility:vis struct $name:ident {
			$($m_visibility:vis $m_name:ident: $m_type:ty),* $(,)?
		}
	) => {
		$visibility struct $name {
			$($m_visibility $m_name: $m_type),*
		}

		impl render::UIElement for $name {
			fn render<'a>(&'a self, ui_pass: &mut render::UIPass<'a>) {
				$(self.$m_name.render(ui_pass);)*
			}

			fn collect<'a>(&'a self, collector: &mut render::UICollector<'a>) {
				$(self.$m_name.collect(collector);)*
			}
		}

		impl ui::Element for $name {
			type Event = $event;

			fn inside(&self, position: math::Vector<2, f32>) -> bool {
				$(self.$m_name.inside(position) )||*
			}

			fn resize(&mut self, state: &(impl render::Has<render::State> + render::Has<render::UIState>), rect: ui::Rect) {
				$(self.$m_name.resize(state, rect);)*
			}

			fn click(&mut self, position: Vector<2, f32>) -> Option<Self::Event> {
				None$(.or_else(|| self.$m_name.click(position)))*
			}
			fn hover(&mut self, position: Vector<2, f32>) -> Option<Self::Event> {
				None$(.or_else(|| self.$m_name.hover(position)))*
			}

		}
	};
}
