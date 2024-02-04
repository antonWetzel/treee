use clap::Parser;

fn main() {
	match run() {
		Ok(()) => {},
		Err(err) => eprintln!("Error: {}", err),
	}
}

fn run() -> Result<(), Error> {
	match Command::parse() {
		Command::Importer(command) => importer::run(command)?,
		Command::Viewer => viewer::run()?,
	}
	Ok(())
}

#[derive(clap::Parser)]
enum Command {
	Importer(importer::Command),
	Viewer,
}

#[derive(thiserror::Error, Debug)]
enum Error {
	#[error(transparent)]
	Import(#[from] importer::Error),

	#[error(transparent)]
	Viewer(#[from] viewer::Error),
}
