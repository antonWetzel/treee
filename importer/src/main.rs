mod cache;
mod calculations;
mod level_of_detail;
mod point;
mod progress;
mod segment;
mod tree;
mod writer;

use std::num::NonZeroU32;

use las::Read;
use math::{Vector, X, Y, Z};
use progress::Progress;
use rayon::prelude::*;
use thiserror::Error;
use writer::Writer;

use tree::Tree;

use crate::{cache::Cache, progress::Stage, segment::Segmenter};

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
	let input = rfd::FileDialog::new()
		.set_title("Select Input File")
		.add_filter("Input File", &["las", "laz"])
		.pick_file()
		.ok_or(ImporterError::NoInputFile)?;

	let output = rfd::FileDialog::new()
		.set_title("Select Output Folder")
		.pick_folder()
		.ok_or(ImporterError::NoOutputFolder)?;

	let stage = Stage::new("Unpacking");

	Writer::setup(&output)?;

	let mut reader = las::Reader::from_path(&input).expect("Unable to open reader");
	let header_min = reader.header().bounds().min;
	let header_max = reader.header().bounds().max;
	let min = Vector::new([header_min.x, header_min.z, -header_max.y]);
	let max = Vector::new([header_max.x, header_max.z, -header_min.y]);
	let diff = max - min;
	let pos = min + diff / 2.0;
	let progress_points = reader.header().number_of_points() / IMPORT_PROGRESS_SCALE;

	stage.finish();

	let mut progress = Progress::new("Import", progress_points as usize);

	let mut segmenter = Segmenter::new((min - pos).map(|v| v as f32), (max - pos).map(|v| v as f32));

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
					progress.step();
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

	progress.finish();

	let stage = Stage::new("Segmenting");
	let segments = segmenter.result();
	stage.finish();

	let mut progress = Progress::new("Calculate", progress_points as usize);

	let mut cache = Cache::new(1024);
	let mut tree = Tree::new(
		(min - pos).map(|v| v as f32),
		diff[X].max(diff[Y]).max(diff[Z]) as f32,
	);

	let (sender, reciever) = crossbeam::channel::bounded(2048);
	let segment_properties = ["trunk", "crown"];
	let (segment_values, _) = rayon::join(
		|| {
			let vec = segments
				.into_par_iter()
				.enumerate()
				.map(|(index, segment)| {
					let index = NonZeroU32::new(index as u32 + 1).unwrap();
					let (points, information) = calculations::calculate(segment.points(), index);
					sender.send((points, index)).unwrap();

					[information.trunk_height, information.crown_height]
				})
				.flatten()
				.collect();
			drop(sender);
			vec
		},
		|| {
			let mut counter = 0;
			for (points, segment) in reciever {
				Writer::save_segment(&output, segment, &points);
				for point in points {
					tree.insert(point, &mut cache);
					counter += 1;
					if counter >= IMPORT_PROGRESS_SCALE {
						progress.step();
						counter -= IMPORT_PROGRESS_SCALE;
					}
				}
			}
		},
	);

	progress.finish();

	let stage = Stage::new("Save Project");

	let properties = ["slice", "sub_index", "curve"];
	let (tree, project) = tree.flatten(
		&properties,
		&segment_properties,
		segment_values,
		input.display().to_string(),
		cache,
	);

	// todo: earlier check if output is empty
	let writer = Writer::new(output, &project)?;

	stage.finish();

	tree.save(writer);

	Ok(())
}

fn main() {
	match import() {
		Ok(()) => {},
		Err(err) => eprintln!("Error: {}", err),
	}
}
