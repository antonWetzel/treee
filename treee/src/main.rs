use pollster::FutureExt;

fn main() {
	simple_logger::SimpleLogger::new()
		.with_level(log::LevelFilter::Info)
		.init()
		.unwrap();
	match treee::try_main().block_on() {
		Ok(()) => {},
		Err(err) => println!("Error: {}", err),
	}
}
