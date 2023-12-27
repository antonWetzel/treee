use render::Has;


pub struct State {
	state: render::State,
	pointcloud: render::PointCloudState,
	mesh: render::MeshState,
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


impl Has<render::MeshState> for State {
	fn get(&self) -> &render::MeshState {
		&self.mesh
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
			mesh: render::MeshState::new(&state),
			ui: render::UIState::new(&state),
			state,
		}
	}
}
