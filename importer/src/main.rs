use clap::Parser;
use importer::*;

fn main() -> Result<(), Error> {
	run(Command::parse())
}
