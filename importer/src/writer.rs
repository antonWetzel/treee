use common::Project;
use std::{
	io::{BufWriter, Write},
	num::NonZeroU32,
	path::{Path, PathBuf},
};

use crate::{calculations::SegmentInformation, point, ImporterError, Statistics};

pub struct Writer {
	path: PathBuf,
	pub points: common::DataFile<render::Point>,
	pub slice: common::DataFile<u32>,
	pub curve: common::DataFile<u32>,
	pub height: common::DataFile<u32>,
	pub segment: common::DataFile<u32>,
}

impl Writer {
	pub fn setup(path: &Path) -> Result<(), ImporterError> {
		if path.is_file() {
			return Err(ImporterError::OutputFolderIsFile);
		}
		if path.is_dir() {
			let mut project_path = path.to_path_buf();
			project_path.push("project.epc");
			if path.read_dir().into_iter().flatten().next().is_some() && !project_path.exists() {
				return Err(ImporterError::OutputFolderIsNotEmpty);
			}
			std::fs::remove_dir_all(path).unwrap();
		}
		std::fs::create_dir_all(path).unwrap();
		Ok(())
	}

	pub fn new(mut path: PathBuf, project: &Project) -> Result<Self, ImporterError> {
		let size = project.root.index as usize + 1;

		path.push("project.epc");
		project.save(&path);

		path.set_file_name("points.data");
		let points = common::DataFile::new(size, &path);

		path.set_file_name("slice.data");
		let slice = common::DataFile::new(size, &path);

		path.set_file_name("curve.data");
		let curve = common::DataFile::new(size, &path);

		path.set_file_name("height.data");
		let height = common::DataFile::new(size, &path);

		path.set_file_name("segment.data");
		let segment = common::DataFile::new(size, &path);

		Ok(Self {
			points,
			slice,
			curve,
			height,
			segment,
			path,
		})
	}

	pub fn save(&mut self, index: u32, points: &[render::Point]) {
		self.points.save(index as usize, points);
	}

	pub fn save_statisitcs(&mut self, statistics: Statistics) {
		self.path.set_file_name("statistics.json");
		let file = std::fs::File::create(&self.path).unwrap();
		serde_json::to_writer_pretty(file, &statistics).unwrap();
	}

	pub fn save_segment(path: &Path, segment: NonZeroU32, data: &[point::Point], information: SegmentInformation) {
		let mut path = path.to_path_buf();
		path.push(format!("segments/{}", segment));
		std::fs::create_dir_all(&path).unwrap();
		path.push("points.data");
		let mut points = BufWriter::new(
			std::fs::OpenOptions::new()
				.write(true)
				.create(true)
				.open(&path)
				.unwrap(),
		);

		path.set_file_name("slice.data");
		let mut slice = BufWriter::new(
			std::fs::OpenOptions::new()
				.write(true)
				.create(true)
				.open(&path)
				.unwrap(),
		);

		path.set_file_name("height.data");
		let mut height = BufWriter::new(
			std::fs::OpenOptions::new()
				.write(true)
				.create(true)
				.open(&path)
				.unwrap(),
		);

		path.set_file_name("curve.data");
		let mut curve = BufWriter::new(
			std::fs::OpenOptions::new()
				.write(true)
				.create(true)
				.open(&path)
				.unwrap(),
		);
		path.set_file_name("segment.data");
		let mut segment_values = BufWriter::new(
			std::fs::OpenOptions::new()
				.write(true)
				.create(true)
				.open(&path)
				.unwrap(),
		);

		for point in data {
			points
				.write_all(bytemuck::cast_slice(&[point.render]))
				.unwrap();
			slice
				.write_all(bytemuck::cast_slice(&[point.slice]))
				.unwrap();
			height
				.write_all(bytemuck::cast_slice(&[point.height]))
				.unwrap();
			curve
				.write_all(bytemuck::cast_slice(&[point.curve]))
				.unwrap();
			segment_values
				.write_all(bytemuck::cast_slice(&[segment.get()]))
				.unwrap();
		}

		let information = common::Segment::new(
			[
				("Trunk".into(), information.trunk_height),
				("Crown".into(), information.crown_height),
			]
			.into_iter()
			.collect(),
		);
		path.set_file_name("segment.information");
		information.save(&path);
	}
}
