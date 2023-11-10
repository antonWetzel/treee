#[macro_export]
macro_rules! Collection {
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

			fn bounding_rect(&self) -> ui::Rect {
				ui::Rect {
					min: [f32::MAX, f32::MAX].into(),
					max: [f32::MIN, f32::MIN].into(),
				}$(.merge(self.$m_name.bounding_rect()))*
			}

			fn resize(&mut self, state: &impl ui::State, rect: ui::Rect) {
				$(self.$m_name.resize(state, rect);)*
			}

			fn click(&mut self, state: &impl ui::State, position: Vector<2, f32>) -> Option<Self::Event> {
				None$(.or_else(|| self.$m_name.click(state, position)))*
			}
			fn hover(&mut self, state: &impl ui::State, position: Vector<2, f32>, pressed: bool) -> Option<Self::Event> {
				None$(.or_else(|| self.$m_name.hover(state, position, pressed)))*
			}

		}
	};
}

#[macro_export]
macro_rules! Stack {
	(
		type Event = $event:ty;

		const DIRECTION = $dir:expr;

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

			fn bounding_rect(&self) -> ui::Rect {
				ui::Rect {
					min: [f32::MAX, f32::MAX].into(),
					max: [f32::MIN, f32::MIN].into(),
				}$(.merge(self.$m_name.bounding_rect()))*
			}

			fn resize(&mut self, state: &impl ui::State, mut rect: ui::Rect) {

				$(
					self.$m_name.resize(state, rect);
					rect.min[$dir] = self.$m_name.bounding_rect().max[$dir];
				)*
			}

			fn click(&mut self, state: &impl ui::State, position: Vector<2, f32>) -> Option<Self::Event> {
				None$(.or_else(|| self.$m_name.click(state, position)))*
			}
			fn hover(&mut self, state: &impl ui::State, position: Vector<2, f32>, pressed: bool) -> Option<Self::Event> {
				None$(.or_else(|| self.$m_name.hover(state, position, pressed)))*
			}

		}
	}
}
