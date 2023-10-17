mod calculations;
mod data_point;
mod progress;
mod tree;
mod writer;

use data_point::DataPoint;
use las::Read;
use math::{Vector, X, Y, Z};
use thiserror::Error;
use writer::Writer;

use tree::Tree;

use crate::progress::Progress;

#[derive(Error, Debug)]
enum ImporterError {
	#[error("no input file")]
	NoInputFile,
	#[error("no output folder")]
	NoOutputFolder,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let input = rfd::FileDialog::new()
		.set_title("Select Input File")
		.add_filter("Input File", &["las", "laz"])
		.pick_file()
		.ok_or(ImporterError::NoInputFile)?;

	let output = rfd::FileDialog::new()
		.set_title("Select Output Folder")
		.pick_folder()
		.ok_or(ImporterError::NoOutputFolder)?;

	let mut reader = las::Reader::from_path(&input).expect("Unable to open reader");

	let header_min = reader.header().bounds().min;
	let header_max = reader.header().bounds().max;
	let min = Vector::new([header_min.x, header_min.z, -header_max.y]);
	let max = Vector::new([header_max.x, header_max.z, -header_min.y]);
	let diff = max - min;
	let pos = min + diff / 2.0;

	let mut writer = Writer::new(output);
	let mut tree = Tree::new(
		&mut writer,
		(min - pos).map(|v| v as f32),
		diff[X].max(diff[Y]).max(diff[Z]) as f32,
	);

	let (sender, reciever) = crossbeam::channel::bounded(4);
	let mut progress = Progress::new("Import".into(), reader.header().number_of_points() as usize);
	std::thread::spawn(move || {
		for point in reader.points().map(|p| p.expect("Unable to read point")) {
			sender.send(point).unwrap();
		}
	});

	for point in reciever {
		let res = DataPoint {
			position: [
				(point.x - pos[X]) as f32,
				(point.z - pos[Y]) as f32,
				(-point.y - pos[Z]) as f32,
			]
			.into(),
			value: {
				let b = (point.z - min[Y]) / diff[Y];
				(b * (u32::MAX - 1) as f64) as u32
			},
		};
		tree.insert(res, &mut writer);
		progress.increase();
	}

	let (tree, project) = tree.flatten();
	writer.setup_property("height");
	writer.save_project(&project);

	let heigt_calculator = HeightCalculator::new((min[Y] - pos[Y]) as f32, (max[Y] - pos[Y]) as f32);
	tree.calculate(&mut writer, &project, &heigt_calculator);

	Ok(())
}

pub struct HeightCalculator {
	min: f32,
	diff: f32,
}

impl HeightCalculator {
	pub fn new(min: f32, max: f32) -> Self {
		Self { min, diff: max - min }
	}

	pub fn calculate(&self, index: usize, points: &[render::Point]) -> u32 {
		let height = points[index].position[Y];
		let b = (height - self.min) / self.diff;
		(b * (u32::MAX - 1) as f32) as u32
	}
}
