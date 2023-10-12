use pollster::FutureExt;
use state::State;

mod camera;
mod game;
mod interface;
mod loaded_manager;
mod lod;
mod state;
mod tree;

fn main() {
	let mut args = std::env::args().rev().collect::<Vec<_>>();
	args.pop();
	let path = args.pop().unwrap();

	let (state, runner) = render::State::new().block_on();
	let state = State::new(state);
	let state = Box::leak(Box::new(state));

	let mut game = game::Game::new(state, path, &runner);
	let code = runner.run(&mut game);
	std::process::exit(code);
}
