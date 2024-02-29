use viewer::*;

fn main() -> Result<(), Error> {
	env_logger::init();

	let mut runner = Runner::new()?;
	run(&mut runner)
}
