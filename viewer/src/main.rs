use std::error::Error;

use pollster::FutureExt;
use state::State;

mod camera;
mod game;
mod interface;
mod loaded_manager;
mod lod;
mod state;
mod tree;

#[derive(Debug)]
enum ViewerError {
	NoFile,
	Exit(i32),
}

impl std::fmt::Display for ViewerError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", self)
	}
}

impl Error for ViewerError {}

fn main() -> Result<(), Box<dyn Error>> {
	let path = rfd::FileDialog::new()
		.add_filter("Project File", &["epc"])
		.pick_file()
		.ok_or(ViewerError::NoFile)?;

	let (state, runner) = render::State::new().block_on();
	let state = State::new(state);
	let state = Box::leak(Box::new(state));

	let mut game = game::Game::new(state, path, &runner);
	match runner.run(&mut game) {
		0 => Ok(()),
		code => Err(ViewerError::Exit(code))?,
	}
}
