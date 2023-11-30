use pollster::FutureExt;
use state::State;
use thiserror::Error;

mod camera;
mod game;
mod interface;
mod loaded_manager;
mod lod;
mod segment;
mod state;
mod tree;

#[derive(Debug, Error)]
enum ViewerError {
	#[error("no file")]
	NoFile,
	#[error("{0}")]
	RenderError(#[from] render::RenderError),
}

fn main() -> Result<(), ViewerError> {
	let path = rfd::FileDialog::new()
		.set_title("Select Project File")
		.add_filter("Project File", &["epc"])
		.pick_file()
		.ok_or(ViewerError::NoFile)?;

	let (state, runner) = render::State::new().block_on()?;
	let state = State::new(state);
	let state = Box::leak(Box::new(state));

	let mut game = game::Game::new(state, path, &runner);
	runner.run(&mut game)?;

	Ok(())
}
