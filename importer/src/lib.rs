mod cache;
mod calculations;
mod laz;
mod level_of_detail;
mod point;
mod progress;
mod segment;
mod tree;
mod writer;

use std::{num::NonZeroU32, ops::Not, path::PathBuf};

use point::PointsCollection;
use progress::Progress;
use project::Property;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use writer::Writer;

use tree::Tree;

use crate::{
	cache::Cache,
	progress::Stage,
	segment::{Segment, Segmenter},
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("No input file")]
	NoInputFile,

	#[error("No output folder")]
	NoOutputFolder,

	#[error(transparent)]
	InvalidFile(#[from] std::io::Error),

	#[error("Corrupt file")]
	CorruptFile,

	#[error(transparent)]
	LasZipError(#[from] ::laz::LasZipError),

	#[error("Output folder is file")]
	OutputFolderIsFile,

	#[error("Output folder is not empty")]
	OutputFolderIsNotEmpty,

	#[error("Atleast two Threads are required")]
	NotEnoughThreads,
}

#[derive(clap::Args)]
pub struct Settings {
	// Single tree, don't segment data.
	#[arg(long, default_value_t = false)]
	single_tree: bool,

	/// Minimum size for segments. Segments with less points are removed.
	#[arg(long, default_value_t = 100)]
	min_segment_size: usize,

	/// Width of the horizontal slice in meters.
	#[arg(long, default_value_t = 1.0)]
	segmenting_slice_width: f32,

	/// Distance to combine segments in meters.
	#[arg(long, default_value_t = 1.0)]
	segmenting_max_distance: f32,

	/// Height in meters to calculate the trunk diameter.
	#[arg(long, default_value_t = 1.3)]
	trunk_diameter_height: f32,

	/// Difference in meters between the trunk diameter and the start diameter of the crown.
	#[arg(long, default_value_t = 1.0)]
	crown_diameter_difference: f32,

	/// Total range for included points in the calculation.
	#[arg(long, default_value_t = 0.2)]
	trunk_diameter_range: f32,

	/// Scale for a slice to be considered ground instead of trunk relative to the smallest area.
	#[arg(long, default_value_t = 1.5)]
	ground_min_area_scale: f32,

	/// Maximum height to search for the ground.
	#[arg(long, default_value_t = 1.0)]
	ground_max_search_height: f32,

	/// Maximum count for neighbors search.
	#[arg(long, default_value_t = 31)]
	neighbors_count: usize,

	/// Slice width for segment expansion calculation.
	#[arg(long, default_value_t = 0.1)]
	calculations_slice_width: f32,

	/// Maximum count for neighbors search.
	#[arg(long, default_value_t = 1.0)]
	neighbors_max_distance: f32,

	/// Scale for the size of the combined point.
	#[arg(long, default_value_t = 0.95)]
	lod_size_scale: f32,

	/// PROJ location transformation.
	/// Can be empty for no transformation.
	#[arg(long, default_value = "+proj=utm +ellps=GRS80 +zone=32")]
	proj_location: String,
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

fn import(settings: Settings, input: PathBuf, output: PathBuf) -> Result<(), Error> {
	let mut cache = Cache::new(4_000_000_000);
	let mut statistics = Statistics::default();
	let stage = Stage::new("Setup Files");

	Writer::setup(&output)?;

	let laz = laz::Laz::new(&input)?;
	let min = laz.min;
	let max = laz.max;
	let world_offset = laz.world_offset;
	let diff = max - min;
	let total_points = laz.total;
	statistics.source_points = total_points;

	statistics.times.setup = stage.finish();

	let mut progress = Progress::new("Import", total_points);

	let (sender, reciever) = crossbeam::channel::bounded(4);

	let (import_result, segments) = rayon::join(
		|| {
			laz.read(|chunk| sender.send(chunk).unwrap())?;
			drop(sender);
			Result::<(), Error>::Ok(())
		},
		|| {
			if settings.single_tree {
				let segment = cache.new_entry();
				for chunk in reciever {
					let l = chunk.length();
					cache.add_chunk(&segment, chunk.read());
					progress.step_by(l);
				}
				statistics.times.import = progress.finish();
				vec![Segment::new(cache.read(segment))]
			} else {
				let mut segmenter = Segmenter::new(min, max, &mut cache, &settings);
				for chunk in reciever {
					let l = chunk.length();
					for point in chunk {
						segmenter.add_point(point, &mut cache);
					}
					progress.step_by(l);
				}
				statistics.times.import = progress.finish();
				let mut segments = segmenter.segments(&mut statistics, &mut cache);
				segments.shuffle(&mut rand::thread_rng());
				segments
			}
		},
	);
	import_result?;
	statistics.segments = segments.len();

	let mut progress = Progress::new("Calculate", total_points);

	let mut tree = Tree::new(min, diff.x.max(diff.y).max(diff.z));
	let segments_information = vec![
		String::from("Total height"),
		String::from("Trunk height"),
		String::from("Crown height"),
		String::from("Trunk diameter"),
		String::from("Crown diameter"),
		String::from("Latitude"),
		String::from("Longitude"),
		String::from("Elevation"),
	];

	let projection = settings.proj_location.is_empty().not().then(|| {
		let from = proj4rs::Proj::from_proj_string(&settings.proj_location).unwrap();
		let to = proj4rs::Proj::from_proj_string("+proj=latlong +ellps=GRS80").unwrap();
		(from, to)
	});

	let (sender, reciever) = crossbeam::channel::bounded(2);
	let (_, segment_values) = rayon::join(
		|| {
			segments
				.into_par_iter()
				.enumerate()
				.for_each(|(index, segment)| {
					let index = NonZeroU32::new(index as u32 + 1).unwrap();
					let (points, information) = calculations::calculate(
						segment.points(),
						index,
						&settings,
						&projection,
						world_offset,
					);
					sender.send((points, index, information)).unwrap();
				});
			drop(sender);
		},
		|| {
			let mut path = output.clone();
			path.push("segments");
			std::fs::create_dir(&path).unwrap();
			let mut segment_writer = Writer::new(path, statistics.segments);
			let mut segment_values =
				vec![project::Value::Percent(0.0); statistics.segments * segments_information.len()];
			for (points, segment, information) in reciever {
				let collection = PointsCollection::from_points(&points);
				segment_writer.save(segment.get() as usize - 1, &collection);
				let offset = (segment.get() - 1) as usize * segments_information.len();
				segment_values[offset + 0] = information.total_height;
				segment_values[offset + 1] = information.trunk_height;
				segment_values[offset + 2] = information.crown_height;
				segment_values[offset + 3] = information.trunk_diameter;
				segment_values[offset + 4] = information.crown_diameter;
				segment_values[offset + 5] = information.latitude;
				segment_values[offset + 6] = information.longitude;
				segment_values[offset + 7] = information.elevation;
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

	let properties = vec![
		Property {
			storage_name: "segment".into(),
			display_name: "Segment".into(),
			max: statistics.segments as u32,
		},
		Property {
			storage_name: "height".into(),
			display_name: "Height".into(),
			max: u32::MAX,
		},
		Property {
			storage_name: "slice".into(),
			display_name: "Expansion".into(),
			max: u32::MAX,
		},
		Property {
			storage_name: "curve".into(),
			display_name: "Curvature".into(),
			max: u32::MAX,
		},
		Property {
			storage_name: "classification".into(),
			display_name: "Classification".into(),
			max: 3,
		},
	];

	let (tree, project) = tree.flatten(
		properties,
		input.display().to_string(),
		cache,
		segments_information,
		segment_values,
	);

	let mut writer = Writer::new(output, project.root.index as usize + 1);
	writer.save_project(&project);

	statistics.times.project = stage.finish();

	tree.save(writer, &settings, statistics);

	Ok(())
}
