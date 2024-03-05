use std::ops::Deref;

pub struct State {
	pub state: render::State,
	pub pointcloud: render::PointCloudState,
	pub mesh: render::MeshState,
	pub mesh_line: render::MeshState,
	pub lines: render::LinesState,
}

impl State {
	pub fn new(state: render::State) -> Self {
		Self {
			pointcloud: render::PointCloudState::new(&state),
			mesh: render::MeshState::new(&state),
			mesh_line: render::MeshState::new_as_lines(&state),
			lines: render::LinesState::new(&state),
			state,
		}
	}
}

impl Deref for State {
	type Target = render::State;

	fn deref(&self) -> &Self::Target {
		&self.state
	}
}
