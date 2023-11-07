mod cache;
mod calculations;
mod level_of_detail;
mod point;
mod segment;
mod tree;
mod writer;

use std::{num::NonZeroU32, time::Duration};

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
	let spinner = ProgressBar::new(0);
	spinner.set_style(ProgressStyle::with_template("{prefix:15} [{elapsed_precise}] {spinner:.blue}").unwrap());

	let input = rfd::FileDialog::new()
		.set_title("Select Input File")
		.add_filter("Input File", &["las", "laz"])
		.pick_file()
		.ok_or(ImporterError::NoInputFile)?;

	let output = rfd::FileDialog::new()
		.set_title("Select Output Folder")
		.pick_folder()
		.ok_or(ImporterError::NoOutputFolder)?;

	// create writer early to check if the folder is empty
	let writer = Writer::new(output)?;

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

	let mut segmenter = Segmenter::new(0.0);

	let (sender, reciever) = crossbeam::channel::bounded(2048);
	rayon::join(
		|| {
			let mut counter = 0;
			//skips invalid points without error or warning
			for point in reader.points().flatten() {
				sender
					.send(Vector::new([point.x, point.z, -point.y]))
					.unwrap();
				counter += 1;
				if counter >= IMPORT_PROGRESS_SCALE {
					progress.inc(1);
					counter -= IMPORT_PROGRESS_SCALE;
				}
			}
			drop(sender);
		},
		|| {
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
		},
	);

	let segments = segmenter.result();
	progress.finish();
	println!();

	progress.reset();
	progress.set_length(progress_points);
	progress.set_prefix("Calculate:");
	progress.enable_steady_tick(Duration::from_micros(100));

	let mut cache = Cache::new();
	let mut tree = Tree::new(
		(min - pos).map(|v| v as f32),
		diff[X].max(diff[Y]).max(diff[Z]) as f32,
	);

	let (sender, reciever) = crossbeam::channel::bounded(2048);
	let segment_properties = ["segment", "random"];
	let mut segment_values = Vec::with_capacity(segment_properties.len() * segments.len());
	rayon::join(
		|| {
			for (index, segment) in segments.into_iter().enumerate() {
				let points = calculations::calculate(segment.points());
				let segment = NonZeroU32::new(index as u32 + 1).unwrap();
				for point in points {
					sender.send((point, segment)).unwrap();
				}
				segment_values.push(common::Value::Index(segment));
				segment_values.push(common::Value::Percent(rand::random()));
			}
			drop(sender);
		},
		|| {
			let mut counter = 0;
			for (point, segment) in reciever {
				tree.insert(point, segment, &mut cache);
				counter += 1;
				if counter >= IMPORT_PROGRESS_SCALE {
					progress.inc(1);
					counter -= IMPORT_PROGRESS_SCALE;
				}
			}
		},
	);

	progress.finish();
	println!();

	spinner.reset();
	spinner.set_prefix("Save Project:");
	spinner.tick();
	spinner.enable_steady_tick(Duration::from_millis(100));

	let properties = ["slice", "sub_index", "curve"];

	for property in properties {
		writer.setup_property(property);
	}

	let (tree, project) = tree.flatten(
		&properties,
		&segment_properties,
		segment_values,
		input.display().to_string(),
		cache,
	);
	writer.save_project(&project);
	spinner.disable_steady_tick();
	spinner.finish();
	println!();

	tree.save(&writer, progress);

	Ok(())
}

fn main() {
	match import() {
		Ok(()) => {},
		Err(err) => eprintln!("Error: {}", err),
	}
}
