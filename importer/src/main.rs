mod calculations;
mod data_point;
mod tree;
mod writer;

use data_point::DataPoint;
use indicatif::{ProgressBar, ProgressStyle};
use las::Read;
use math::{Vector, X, Y, Z};
use thiserror::Error;
use writer::Writer;

use tree::Tree;

const IMPORT_PROGRESS_SCALE: u64 = 10_000;

#[derive(Error, Debug)]
enum ImporterError {
	#[error("no input file")]
	NoInputFile,
	#[error("no output folder")]
	NoOutputFolder,
}

fn main() -> Result<(), ImporterError> {
	let progress = ProgressBar::new(0);
	progress.set_style(
		ProgressStyle::with_template(
			"{prefix:12} [{elapsed_precise}] {wide_bar:.cyan/blue} {human_pos:>12}/{human_len:12} {eta_precise}",
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

	progress.reset();
	progress.set_length(reader.header().number_of_points() / IMPORT_PROGRESS_SCALE);
	progress.set_prefix("Import:");

	let (sender, reciever) = crossbeam::channel::bounded(512);
	//parallel over files?
	let reader_progress = progress.clone();
	std::thread::spawn(move || {
		let mut counter = 0;
		for point in reader.points().map(|p| p.expect("Unable to read point")) {
			sender.send(point).unwrap();
			counter += 1;
			if counter >= IMPORT_PROGRESS_SCALE {
				reader_progress.inc(1);
				counter -= IMPORT_PROGRESS_SCALE;
			}
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
	}
	progress.finish();
	println!();

	let heigt_calculator = HeightCalculator::new((min[Y] - pos[Y]) as f32, (max[Y] - pos[Y]) as f32);
	let inverse_calculator = InverseHeightCalculator::new((min[Y] - pos[Y]) as f32, (max[Y] - pos[Y]) as f32);
	let calculators: &[&dyn Calculator] = &[&heigt_calculator, &inverse_calculator];
	for calculator in calculators {
		writer.setup_property(calculator.name());
	}

	let (tree, project) = tree.flatten(calculators);
	writer.save_project(&project);

	tree.calculate(&writer, &project, progress, calculators);

	Ok(())
}

pub trait SimpleCalculator: Send + Sync {
	fn name(&self) -> &str;
	fn calculate(&self, index: usize, points: &[render::Point]) -> u32;
}

impl<T: SimpleCalculator> Calculator for T {
	fn name(&self) -> &str {
		<Self as SimpleCalculator>::name(&self)
	}
	fn calculate(&self, points: &[render::Point]) -> Vec<u32> {
		let mut values = Vec::with_capacity(points.len());
		for i in 0..points.len() {
			values.push(self.calculate(i, points));
		}
		values
	}
}
pub trait Calculator: Send + Sync {
	fn name(&self) -> &str;
	fn calculate(&self, points: &[render::Point]) -> Vec<u32>;
}

pub struct HeightCalculator {
	min: f32,
	diff: f32,
}

impl HeightCalculator {
	pub fn new(min: f32, max: f32) -> Self {
		Self { min, diff: max - min }
	}
}

impl SimpleCalculator for HeightCalculator {
	fn name(&self) -> &str {
		"height"
	}

	fn calculate(&self, index: usize, points: &[render::Point]) -> u32 {
		let height = points[index].position[Y];
		let b = (height - self.min) / self.diff;
		(b * (u32::MAX - 1) as f32) as u32
	}
}

pub struct InverseHeightCalculator {
	min: f32,
	diff: f32,
}

impl InverseHeightCalculator {
	pub fn new(min: f32, max: f32) -> Self {
		Self { min, diff: max - min }
	}
}

impl SimpleCalculator for InverseHeightCalculator {
	fn name(&self) -> &str {
		"inverse_height"
	}

	fn calculate(&self, index: usize, points: &[render::Point]) -> u32 {
		let height = points[index].position[Y];
		let b = (height - self.min) / self.diff;
		u32::MAX - (b * (u32::MAX - 1) as f32) as u32
	}
}
