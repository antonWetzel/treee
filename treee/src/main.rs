#![windows_subsystem = "windows"]

use pollster::FutureExt;

/// Main function for native OS.
fn main() {
	simple_logger::SimpleLogger::new()
		.with_level(log::LevelFilter::Warn)
		.init()
		.unwrap();
	treee::try_main(|err| println!("Error: {}", err)).block_on();
}
