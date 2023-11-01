mod cache;
mod calculations;
mod level_of_detail;
mod point;
mod segment;
mod tree;
mod writer;

use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use las::Read;
use math::{Vector, X, Y, Z};
use thiserror::Error;
use writer::Writer;

use tree::Tree;

use crate::{cache::Cache, segment::Segmenter};

const IMPORT_PROGRESS_SCALE: u64 = 10_000;

#[derive(Error, Debug)]
pub enum ImporterError {
	#[error("No input file")]
	NoInputFile,
	#[error("No output folder")]
	NoOutputFolder,

	#[error(transparent)]
	InvalidFile(#[from] Box<las::Error>),

	#[error("Output folder is file")]
	OutputFolderIsFile,

	#[error("Output folder is not empty")]
	OutputFolderIsNotEmpty,
}

fn import() -> Result<(), ImporterError> {
	let progress = ProgressBar::new(0);
	progress.set_style(
		ProgressStyle::with_template(
			"{prefix:15} [{elapsed_precise}] {wide_bar:.cyan/blue} {human_pos:>12}/{human_len:12} {eta_precise}",
		)
		.unwrap(),
	);

	let input = rfd::FileDialog::new()
		.set_title("Select Input File")
		.add_filter("Input File", &["las", "laz"])
		.pick_file()
		.ok_or(ImporterError::NoInputFile)?;

	let output = rfd::FileDialog::new()
		.set_title("Select Output Folder")
		.pick_folder()
		.ok_or(ImporterError::NoOutputFolder)?;

	let spinner = ProgressBar::new(0);
	spinner.set_style(ProgressStyle::with_template("{prefix:15} [{elapsed_precise}] {spinner:.blue}").unwrap());

	spinner.reset();
	spinner.tick();
	spinner.set_prefix("Unpacking:");
	spinner.enable_steady_tick(Duration::from_millis(100));

	let mut reader = las::Reader::from_path(&input).expect("Unable to open reader");
	let header_min = reader.header().bounds().min;
	let header_max = reader.header().bounds().max;
	let min = Vector::new([header_min.x, header_min.z, -header_max.y]);
	let max = Vector::new([header_max.x, header_max.z, -header_min.y]);
	let diff = max - min;
	let pos = min + diff / 2.0;
	let progress_points = reader.header().number_of_points() / IMPORT_PROGRESS_SCALE;

	spinner.disable_steady_tick();
	spinner.finish();
	println!();

	progress.reset();
	progress.set_length(progress_points);
	progress.set_prefix("Import:");
	progress.inc(0);

	let (sender, reciever) = crossbeam::channel::bounded(2048);
	//parallel over files?
	let reader_progress = progress.clone();
	std::thread::spawn(move || {
		let mut counter = 0;
		//skips invalid points without error or warning
		for point in reader.points().flatten() {
			sender
				.send(Vector::new([point.x, point.z, -point.y]))
				.unwrap();
			counter += 1;
			if counter >= IMPORT_PROGRESS_SCALE {
				reader_progress.inc(1);
				counter -= IMPORT_PROGRESS_SCALE;
			}
		}
	});

	let mut segmenter = Segmenter::new();
	for point in reciever {
		segmenter.add_point(
			[
				(point[X] - pos[X]) as f32,
				(point[Y] - pos[Y]) as f32,
				(point[Z] - pos[Z]) as f32,
			]
			.into(),
		);
	}

	let segments = segmenter.result();
	progress.finish();
	println!();

	progress.reset();
	progress.set_length(progress_points);
	progress.set_prefix("Calculate:");
	progress.enable_steady_tick(Duration::from_micros(100));

	let (sender, reciever) = crossbeam::channel::bounded(2048);
	std::thread::spawn(move || {
		for segment in segments {
			let points = calculations::calculate(segment.points());
			for point in points {
				sender.send(point).unwrap();
			}
		}
	});

	let mut writer = Writer::new(output)?;
	let mut tree = Tree::new(
		&mut writer,
		(min - pos).map(|v| v as f32),
		diff[X].max(diff[Y]).max(diff[Z]) as f32,
	);
	let mut counter = 0;
	let mut cache = Cache::new();
	for point in reciever {
		tree.insert(point, &mut writer, &mut cache);
		counter += 1;
		if counter >= IMPORT_PROGRESS_SCALE {
			progress.inc(1);
			counter -= IMPORT_PROGRESS_SCALE;
		}
	}

	progress.finish();
	println!();

	spinner.reset();
	spinner.set_prefix("Save Project:");
	spinner.tick();
	spinner.enable_steady_tick(Duration::from_millis(100));

	let properties = ["slice"];

	for property in properties {
		writer.setup_property(property);
	}

	let (tree, project) = tree.flatten(&properties, input.display().to_string(), cache);
	writer.save_project(&project);
	spinner.disable_steady_tick();
	spinner.finish();
	println!();

	tree.save(&writer, &project, progress);

	Ok(())
}

fn main() {
	match import() {
		Ok(()) => {},
		Err(err) => eprintln!("Error: {}", err),
	}
}
