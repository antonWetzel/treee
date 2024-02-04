mod camera;
mod game;
mod loaded_manager;
mod lod;
mod segment;
mod state;
mod tree;

use pollster::FutureExt;
use state::State;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("no file")]
	NoFile,
	#[error("{0}")]
	RenderError(#[from] render::RenderError),
}

pub fn run() -> Result<(), Error> {
	simple_logger::SimpleLogger::new()
		.with_level(log::LevelFilter::Info)
		.init()
		.unwrap();
	let path = rfd::FileDialog::new()
		.set_title("Select Project File")
		.add_filter("Project File", &["epc"])
		.pick_file()
		.ok_or(Error::NoFile)?;

	let (state, runner) = render::State::new().block_on()?;
	let state = State::new(state);
	let state = Box::leak(Box::new(state));

	let mut game = game::World::new(state, path, &runner);
	runner.run(&mut game)?;

	Ok(())
}
