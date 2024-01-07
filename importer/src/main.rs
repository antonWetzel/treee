mod cache;
mod calculations;
mod level_of_detail;
mod point;
mod progress;
mod segment;
mod tree;
mod writer;


use std::path::PathBuf;

use clap::Parser;
use las::Read;
use math::{ Vector, X, Y, Z };
use progress::Progress;
use rayon::prelude::*;
use thiserror::Error;
use writer::Writer;

use tree::Tree;

use crate::{ cache::Cache, progress::Stage, segment::Segmenter };


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

	#[error("Atleast two Threads are required")]
	NotEnoughThreads,
}


#[derive(clap::Args)]
pub struct Settings {
	/// Minimum size for segments. Segments with less points are removed.
	#[arg(long, default_value_t = 100)]
	min_segment_size: usize,

	/// Maximum count for neighbors search
	#[arg(long, default_value_t = 31)]
	neighbors_count: usize,

	/// Maximum distance in meters for the neighbors search
	#[arg(long, default_value_t = 1.0)]
	neighbors_max_distance: f32,

	/// Scale for the size of the combined point
	#[arg(long, default_value_t = 0.95)]
	lod_size_scale: f32,
}


#[derive(clap::Parser)]
pub struct Cli {
	/// Input file location. Open File Dialog if not specified.
	input_file: Option<PathBuf>,

	/// Output folder location. Open File Dialog if not specified.
	#[arg(long, short)]
	output_folder: Option<PathBuf>,

	/// Maximal thread count for multithreading. 0 for the amount of logical cores.
	#[arg(long, default_value_t = 4)]
	max_threads: usize,

	#[command(flatten)]
	settings: Settings,
}


fn map_point(point: las::Point, center: Vector<3, f64>) -> Vector<3, f32> {
	(Vector::new([point.x, point.z, -point.y]) - center).map(|v| v as f32)
}


fn import(cli: Cli) -> Result<(), ImporterError> {
	if cli.max_threads == 1 {
		return Err(ImporterError::NotEnoughThreads);
	}
	let input = match cli.input_file {
		Some(file) => file,
		None => rfd::FileDialog::new()
			.set_title("Select Input File")
			.add_filter("Input File", &["las", "laz"])
			.pick_file()
			.ok_or(ImporterError::NoInputFile)?,
	};

	let output = match cli.output_folder {
		Some(folder) => folder,
		None => rfd::FileDialog::new()
			.set_title("Select Output Folder")
			.pick_folder()
			.ok_or(ImporterError::NoOutputFolder)?,
	};
	let settings = cli.settings;

	let stage = Stage::new("Setup Files");

	Writer::setup(&output)?;

	let mut reader = las::Reader::from_path(&input).map_err(Box::new)?;
	let header = reader.header();
	let header_min = header.bounds().min;
	let header_max = header.bounds().max;
	let min = Vector::new([header_min.x, header_min.z, -header_max.y]);
	let max = Vector::new([header_max.x, header_max.z, -header_min.y]);
	let diff = max - min;
	let pos = min + diff / 2.0;
	let progress_points = header.number_of_points() / IMPORT_PROGRESS_SCALE;

	stage.finish();

	let mut progress = Progress::new("Import", progress_points as usize);

	let mut segmenter = Segmenter::new((min - pos).map(|v| v as f32), (max - pos).map(|v| v as f32));

	let (sender, reciever) = crossbeam::channel::bounded(2048);
	rayon::join(
		|| {
			//skips invalid points without error or warning
			for point in reader.points().flatten() {
				sender.send(map_point(point, pos)).unwrap();
			}
			drop(sender);
		},
		|| {
			let mut counter = 0;
			for point in reciever {
				segmenter.add_point(point);
				counter += 1;
				if counter >= IMPORT_PROGRESS_SCALE {
					progress.step();
					counter -= IMPORT_PROGRESS_SCALE;
				}
			}
		},
	);

	progress.finish();

	let progress = Stage::new("Segmenting");
	let segments = segmenter.segments();
	progress.finish();

	let mut progress = Progress::new("Calculate", progress_points as usize);

	let mut cache = Cache::new(1024);
	let mut tree = Tree::new(
		(min - pos).map(|v| v as f32),
		diff[X].max(diff[Y]).max(diff[Z]) as f32,
	);

	let (sender, reciever) = crossbeam::channel::bounded(2048);
	rayon::join(
		|| {
			segments
				.into_par_iter()
				.filter(|segment| segment.length() >= settings.min_segment_size)
				.for_each(|segment| {
					let index = rand::random();
					let (points, information) = calculations::calculate(
						segment.points(),
						index,
						&settings,
					);
					sender.send((points, index, information)).unwrap();
				});
			drop(sender);
		},
		|| {
			let mut counter = 0;
			for (points, segment, information) in reciever {
				Writer::save_segment(&output, segment, &points, information);
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

	let properties = [("segment", "Segment"), ("height", "Height"), ("slice", "Expansion"), ("curve", "Curvature")];
	let (tree, project) = tree.flatten(
		&properties,
		input.display().to_string(),
		cache,
	);

	let writer = Writer::new(output, &project)?;

	stage.finish();

	tree.save(writer, &settings);

	Ok(())
}


fn main() {
	let cli = Cli::parse();
	let res = rayon::ThreadPoolBuilder::new()
		.num_threads(cli.max_threads.min(std::thread::available_parallelism().map(|v| v.get()).unwrap_or(0)))
		.build()
		.unwrap()
		.install(|| import(cli));
	match res {
		Ok(()) => { },
		Err(err) => eprintln!("Error: {}", err),
	}
}
