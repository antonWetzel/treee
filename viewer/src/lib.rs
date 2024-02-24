mod camera;
mod game;
mod loaded_manager;
mod lod;
mod reader;
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
	env_logger::init();

	let path = rfd::FileDialog::new()
		.set_title("Select Project File")
		.add_filter("Project File", &["epc"])
		.pick_file()
		.ok_or(Error::NoFile)?;

	let (mut game, runner) = game::World::new(path).block_on()?;
	runner.run(&mut game)?;

	Ok(())
}
