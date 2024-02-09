mod cache;
mod calculations;
mod level_of_detail;
mod point;
mod progress;
mod segment;
mod tree;
mod writer;

use std::{num::NonZeroU32, path::PathBuf};

use las::Read;
use math::{Vector, X, Y, Z};
use progress::Progress;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use tracing::{span, Level};
use tracing_flame::FlameLayer;
use tracing_subscriber::{layer::SubscriberExt, Registry};
use writer::{SegmentWriter, Writer};

use tree::Tree;

use crate::{cache::Cache, progress::Stage, segment::Segmenter};

#[derive(thiserror::Error, Debug)]
pub enum Error {
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
pub struct Command {
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

pub fn run(command: Command) -> Result<(), Error> {
	// let _guard = {
	// 	let (flame_layer, _guard) = FlameLayer::with_file("./tracing.folded").unwrap();
	// 	let subscriber = Registry::default().with(flame_layer.with_threads_collapsed(true));
	// 	tracing::subscriber::set_global_default(subscriber).expect("Could not set global default");
	// 	_guard
	// };

	// let test = span!(Level::TRACE, "test").entered();
	// {
	// 	let sub = span!(parent: &test, Level::TRACE, "sub").entered();
	// 	std::thread::sleep(std::time::Duration::from_millis(1000));
	// 	sub.exit();
	// }
	// {
	// 	let sub = span!(parent: &test, Level::TRACE, "sub_2").entered();
	// 	std::thread::sleep(std::time::Duration::from_millis(1000));
	// 	sub.exit();
	// }
	// test.exit();
	// std::process::exit(0);

	// #[tracing::instrument(parent = x)]
	// fn test(x: &tracing::Span) {}

	let input = match command.input_file {
		Some(file) => file,
		None => rfd::FileDialog::new()
			.set_title("Select Input File")
			.add_filter("Input File", &["las", "laz"])
			.pick_file()
			.ok_or(Error::NoInputFile)?,
	};

	let output = match command.output_folder {
		Some(folder) => folder,
		None => rfd::FileDialog::new()
			.set_title("Select Output Folder")
			.pick_folder()
			.ok_or(Error::NoOutputFolder)?,
	};

	if command.max_threads == 1 {
		return Err(Error::NotEnoughThreads);
	}

	let settings = command.settings;

	rayon::ThreadPoolBuilder::new()
		.num_threads(command.max_threads)
		.build()
		.unwrap()
		.install(|| import(settings, input, output))
}

fn map_point(point: las::Point, center: Vector<3, f64>) -> Vector<3, f32> {
	(Vector::new([point.x, point.z, -point.y]) - center).map(|v| v as f32)
}

fn import(settings: Settings, input: PathBuf, output: PathBuf) -> Result<(), Error> {
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

	let (sender, reciever) = crossbeam::channel::bounded::<Vec<Vector<3, f32>>>(4);
	let (back_sender, back_reciever) = crossbeam::channel::bounded::<Vec<Vector<3, f32>>>(4);
	rayon::join(
		|| {
			//skips invalid points without error or warning
			let mut points = back_reciever.recv().unwrap();
			for point in reader.points().flatten() {
				points.push(map_point(point, pos));
				if points.len() == points.capacity() {
					sender.send(points).unwrap();
					points = back_reciever.recv().unwrap();
				}
			}
			drop(sender);
			for _ in back_reciever {}
		},
		|| {
			for _ in 0..3 {
				back_sender.send(Vec::with_capacity(2048)).unwrap();
			}
			for mut points in reciever {
				let l = points.len();
				for &point in &points {
					segmenter.add_point(point, &mut cache);
				}
				points.clear();
				back_sender.send(points).unwrap();
				progress.step_by(l);
			}
			drop(back_sender);
		},
	);

	statistics.times.import = progress.finish();

	let mut segments = segmenter.segments(&mut statistics, &mut cache);
	statistics.segments = segments.len();
	segments.shuffle(&mut rand::thread_rng());

	let mut progress = Progress::new("Calculate", total_points);

	let mut tree = Tree::new(
		(min - pos).map(|v| v as f32),
		diff[X].max(diff[Y]).max(diff[Z]) as f32,
	);
	let segments_information = vec![String::from("Crown"), String::from("Trunk")];

	let (sender, reciever) = crossbeam::channel::bounded(2);
	let (_, segment_values) = rayon::join(
		|| {
			segments
				.into_par_iter()
				.enumerate()
				.for_each(|(index, segment)| {
					let index = NonZeroU32::new(index as u32 + 1).unwrap();
					let (points, information) = calculations::calculate(segment.points(), index, &settings);
					sender.send((points, index, information)).unwrap();
				});
			drop(sender);
		},
		|| {
			let mut segment_values =
				vec![common::Value::Percent(0.0); statistics.segments * segments_information.len()];
			for (points, segment, information) in reciever {
				SegmentWriter::new().save_segment(&output, segment, &points);
				let offset = (segment.get() - 1) as usize;
				segment_values[offset * segments_information.len() + 0] = information.trunk_height;
				segment_values[offset * segments_information.len() + 1] = information.crown_height;
				let l = points.len();
				for point in points {
					tree.insert(point, &mut cache);
				}
				progress.step_by(l);
			}
			segment_values
		},
	);

	statistics.times.calculate = progress.finish();

	let stage = Stage::new("Save Project");

	let properties = [
		("segment", "Segment", statistics.segments as u32),
		("height", "Height", u32::MAX),
		("slice", "Expansion", u32::MAX),
		("curve", "Curvature", u32::MAX),
	];

	let (tree, project) = tree.flatten(
		&properties,
		input.display().to_string(),
		cache,
		segments_information,
		segment_values,
	);

	let writer = Writer::new(output, &project)?;

	statistics.times.project = stage.finish();

	tree.save(writer, &settings, statistics);

	Ok(())
}
