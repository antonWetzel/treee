use render::Has;


pub struct State {
	state: render::State,
	pointcloud: render::PointCloudState,
	mesh: render::MeshState,
	lines: render::LinesState,
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


impl Has<render::LinesState> for State {
	fn get(&self) -> &render::LinesState {
		&self.lines
	}
}


impl State {
	pub fn new(state: render::State) -> Self {
		Self {
			pointcloud: render::PointCloudState::new(&state),
			mesh: render::MeshState::new(&state),
			lines: render::LinesState::new(&state),
			state,
		}
	}
}
