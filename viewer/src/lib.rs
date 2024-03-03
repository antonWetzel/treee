mod camera;
mod game;
mod loaded_manager;
mod lod;
mod reader;
mod segment;
mod tree;

use pollster::FutureExt;
use window::State;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("no file")]
	NoFile,
	#[error("{0}")]
	RenderError(#[from] render::RenderError),
}

pub type Runner = render::Runner;

pub fn run(runner: &mut Runner) -> Result<(), Error> {
	let mut game = game::World::new(runner).block_on()?;
	runner.run(&mut game)?;

	Ok(())
}
