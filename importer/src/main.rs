mod calculations;
mod tree;
mod writer;

use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use las::Read;
use math::{Vector, X, Y, Z};
use thiserror::Error;
use writer::Writer;

use tree::Tree;

const IMPORT_PROGRESS_SCALE: u64 = 10_000;

#[derive(Error, Debug)]
pub enum ImporterError {
	#[error("No input file")]
	NoInputFile,
	#[error("No output folder")]
	NoOutputFolder,

	#[error(transparent)]
	InvalidFile(#[from] las::Error),

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

	let mut writer = Writer::new(output)?;
	let mut tree = Tree::new(
		&mut writer,
		(min - pos).map(|v| v as f32),
		diff[X].max(diff[Y]).max(diff[Z]) as f32,
	);

	spinner.disable_steady_tick();
	spinner.finish();
	println!();

	progress.reset();
	progress.set_length(reader.header().number_of_points() / IMPORT_PROGRESS_SCALE);
	progress.set_prefix("Import:");
	progress.inc(0);

	let (sender, reciever) = crossbeam::channel::bounded(512);
	//parallel over files?
	let reader_progress = progress.clone();
	std::thread::spawn(move || {
		let mut counter = 0;
		//skips invalid points without error or warning
		for point in reader.points().flatten() {
			sender.send(point).unwrap();
			counter += 1;
			if counter >= IMPORT_PROGRESS_SCALE {
				reader_progress.inc(1);
				counter -= IMPORT_PROGRESS_SCALE;
			}
		}
	});

	for point in reciever {
		tree.insert(
			[
				(point.x - pos[X]) as f32,
				(point.z - pos[Y]) as f32,
				(-point.y - pos[Z]) as f32,
			]
			.into(),
			&mut writer,
		);
	}
	progress.finish();
	println!();

	spinner.reset();
	spinner.set_prefix("Save Project:");
	spinner.tick();
	spinner.enable_steady_tick(Duration::from_millis(100));

	let heigt_calculator = HeightCalculator::new((min[Y] - pos[Y]) as f32, (max[Y] - pos[Y]) as f32);
	let inverse_calculator = InverseHeightCalculator::new((min[Y] - pos[Y]) as f32, (max[Y] - pos[Y]) as f32);
	let calculators = [&heigt_calculator as &dyn Calculator, &inverse_calculator];
	for calculator in calculators {
		writer.setup_property(calculator.name());
	}

	let (tree, project) = tree.flatten(&calculators);
	writer.save_project(&project);
	spinner.disable_steady_tick();
	spinner.finish();
	println!();

	tree.calculate(&writer, &project, progress, &calculators);

	Ok(())
}

fn main() {
	match import() {
		Ok(()) => {},
		Err(err) => eprintln!("Error: {}", err),
	}
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
