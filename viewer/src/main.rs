use std::error::Error;

use pollster::FutureExt;
use state::State;
use thiserror::Error;

mod camera;
mod game;
mod interface;
mod loaded_manager;
mod lod;
mod state;
mod tree;

#[derive(Debug, Error)]
enum ViewerError {
	#[error("no file")]
	NoFile,
	#[error("unexpected exit code '{0}'")]
	Exit(i32),
}

fn main() -> Result<(), Box<dyn Error>> {
	let path = rfd::FileDialog::new()
		.set_title("Select Project File")
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
