use viewer::*;

fn main() -> Result<(), Error> {
	env_logger::init();

	let mut event_loop = EventLoop::new()?;
	run(&mut event_loop)
}
