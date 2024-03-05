use std::io::Write;

use clap::{CommandFactory, Parser};
use colored::Colorize;

fn main() {
	if std::env::args().len() > 1 {
		cli()
	} else {
		interactive()
	};
}

fn interactive() {
	let mut c = InteractiveCommand::command();
	c.print_help().unwrap();

	let mut event_loop = Option::<viewer::EventLoop>::None;
	loop {
		print!("{}", "\n=> ".bold().green());
		std::io::stdout().flush().unwrap();

		let mut line = String::from("treee ");
		std::io::stdin().read_line(&mut line).unwrap();
		match InteractiveCommand::try_parse_from(line.split_whitespace()) {
			Ok(InteractiveCommand::Importer(command)) => {
				if let Err(err) = importer::run(command) {
					println!("Error: {}", err);
				}
			},
			Ok(InteractiveCommand::Viewer) => {
				let res = match &mut event_loop {
					Some(r) => viewer::run(r),
					None => match viewer::EventLoop::new() {
						Ok(mut r) => {
							let res = viewer::run(&mut r);
							event_loop = Some(r);
							res
						},
						Err(err) => Err(err).map_err(|err| err.into()),
					},
				};
				if let Err(err) = res {
					println!("Error: {}", err);
				}
			},
			Ok(InteractiveCommand::Quit) => break,
			Err(err) => err.print().unwrap(),
		}
	}
}

fn cli() {
	let res = match Command::parse() {
		Command::Importer(command) => importer::run(command).map_err(Error::from),
		Command::Viewer => viewer::EventLoop::new()
			.map_err(viewer::Error::RenderError)
			.and_then(|mut EventLoop| viewer::run(&mut EventLoop))
			.map_err(Error::Viewer),
	};
	if let Err(err) = res {
		println!("Error: {}", err);
	}
}

#[derive(clap::Parser)]
#[command(arg_required_else_help = false)]
enum InteractiveCommand {
	/// Start importer
	Importer(importer::Command),
	/// Start viewer
	Viewer,
	/// Quit application
	Quit,
}

#[derive(clap::Parser)]
enum Command {
	/// Start importer
	Importer(importer::Command),
	/// Start viewer
	Viewer,
}

#[derive(thiserror::Error, Debug)]
enum Error {
	#[error(transparent)]
	Import(#[from] importer::Error),

	#[error(transparent)]
	Viewer(#[from] viewer::Error),
}
