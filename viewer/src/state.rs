use render::Has;

pub struct State {
	state: render::State,
	pointcloud: render::PointCloudState,
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

impl State {
	pub fn new(state: render::State) -> Self {
		Self {
			pointcloud: render::PointCloudState::new(&state),
			state,
		}
	}

	pub fn pointcloud(&self) -> &render::PointCloudState {
		&self.pointcloud
	}
}
