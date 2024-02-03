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
use math::{Vector, X, Y, Z};
use progress::Progress;
use rayon::prelude::*;
use thiserror::Error;
use writer::Writer;

use tree::Tree;

use crate::{cache::Cache, progress::Stage, segment::Segmenter};

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

	/// Width of the horizontal slice in meters
	#[arg(long, default_value_t = 1.0)]
	segmenting_slice_width: f32,

	/// Distance to combine segments in meters
	#[arg(long, default_value_t = 1.0)]
	segmenting_max_distance: f32,

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
	#[arg(long, default_value_t = 0)]
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

	let mut cache = Cache::new(4_000_000_000);
	let mut statistics = Statistics::default();
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
	let total_points = header.number_of_points() as usize;
	statistics.source_points = total_points;

	statistics.times.setup = stage.finish();

	let mut progress = Progress::new("Import", total_points);

	let mut segmenter = Segmenter::new(
		(min - pos).map(|v| v as f32),
		(max - pos).map(|v| v as f32),
		&mut cache,
		&settings,
	);

	let (sender, reciever) = crossbeam::channel::bounded(16);
	rayon::join(
		|| {
			const CHUNK_SIZE: usize = 1024;
			let mut points = Vec::with_capacity(CHUNK_SIZE);
			//skips invalid points without error or warning
			for point in reader.points().flatten() {
				points.push(map_point(point, pos));
				if points.len() >= CHUNK_SIZE {
					sender.send(points).unwrap();
					points = Vec::with_capacity(CHUNK_SIZE);
				}
			}
			drop(sender);
		},
		|| {
			for points in reciever {
				let l = points.len();
				for point in points {
					segmenter.add_point(point, &mut cache);
				}
				progress.step_by(l);
			}
		},
	);

	statistics.times.import = progress.finish();

	let segments = segmenter.segments(&mut statistics, &mut cache);
	statistics.segments = segments.len();

	let mut progress = Progress::new("Calculate", total_points);

	let mut tree = Tree::new(
		(min - pos).map(|v| v as f32),
		diff[X].max(diff[Y]).max(diff[Z]) as f32,
	);

	let (sender, reciever) = crossbeam::channel::bounded(2);
	rayon::join(
		|| {
			segments
				.into_par_iter()
				.filter(|segment| segment.length() >= settings.min_segment_size)
				.for_each(|segment| {
					let index = rand::random();
					let (points, information) = calculations::calculate(segment.points(), index, &settings);
					sender.send((points, index, information)).unwrap();
				});
			drop(sender);
		},
		|| {
			for (points, segment, information) in reciever {
				Writer::save_segment(&output, segment, &points, information);
				let l = points.len();
				for point in points {
					tree.insert(point, &mut cache);
				}
				progress.step_by(l);
			}
		},
	);

	statistics.times.calculate = progress.finish();

	let stage = Stage::new("Save Project");

	let properties = [
		("segment", "Segment"),
		("height", "Height"),
		("slice", "Expansion"),
		("curve", "Curvature"),
	];
	let (tree, project) = tree.flatten(&properties, input.display().to_string(), cache);

	let writer = Writer::new(output, &project)?;

	statistics.times.project = stage.finish();

	tree.save(writer, &settings, statistics);

	Ok(())
}

fn main() {
	let cli = Cli::parse();
	let res = rayon::ThreadPoolBuilder::new()
		.num_threads(cli.max_threads)
		.build()
		.unwrap()
		.install(|| import(cli));
	match res {
		Ok(()) => {},
		Err(err) => eprintln!("Error: {}", err),
	}
}

#[derive(Default, serde::Serialize)]
pub struct Statistics {
	source_points: usize,
	leaf_points: usize,
	branch_points: usize,
	segments: usize,
	times: Times,
}

#[derive(Default, serde::Serialize)]
pub struct Times {
	setup: f32,
	import: f32,
	segment: f32,
	calculate: f32,
	project: f32,
	lods: f32,
}
