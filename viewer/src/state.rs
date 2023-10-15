use render::Has;

pub struct State {
	state: render::State,
	pointcloud: render::PointCloudState,
	ui: render::UIState,
}

impl Has<render::State> for State {
	fn get(&self) -> &render::State {
		&self.state
	}
}

impl Has<render::PointCloudState> for State {
	fn get(&self) -> &render::PointCloudState {
		&self.pointcloud
	}
}

impl Has<render::UIState> for State {
	fn get(&self) -> &render::UIState {
		&self.ui
	}
}

impl State {
	pub fn new(state: render::State) -> Self {
		Self {
			pointcloud: render::PointCloudState::new(&state),
			ui: render::UIState::new(&state, 1080.0),
			state,
		}
	}
}
