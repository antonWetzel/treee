use laz::{
	las::file::{read_vlrs_and_get_laszip_vlr, QuickHeader},
	laszip::ChunkTable,
	record::{LayeredPointRecordDecompressor, RecordDecompressor},
	DecompressionSelection, LazItem, LazVlr, ParLasZipDecompressor,
};
use math::{Vector, X, Y, Z};
use rayon::prelude::*;
use std::{
	fs::File,
	io::{Read, Seek, SeekFrom},
	path::{Path, PathBuf},
};

use crate::Error;

pub struct Laz {
	pub remaining: usize,
	vlr: LazVlr,
	chunks: ChunkTable,
	chunk_index: usize,
	point_length: usize,
	center: Vector<3, f64>,
	offset: Vector<3, f64>,
	scale: Vector<3, f64>,

	path: PathBuf,

	file: File,
	pub min: Vector<3, f32>,
	pub max: Vector<3, f32>,
}

impl Laz {
	pub fn new(path: &Path) -> Result<Self, Error> {
		let mut file = std::fs::File::open(path)?;
		let mut header = las::raw::Header::read_from(&mut file).unwrap(); //todo: remove las dependency
		if header.number_of_point_records == 0 {
			file.seek(SeekFrom::Start(247)).unwrap();
			let mut buffer = [0u8; 4];
			file.read_exact(&mut buffer).unwrap();
			header.number_of_point_records = u32::from_le_bytes(buffer);
		}
		file.seek(SeekFrom::Start(header.header_size as u64))
			.unwrap();
		let vlr = read_vlrs_and_get_laszip_vlr(
			&mut file,
			&QuickHeader {
				major: header.version.major,
				minor: header.version.minor,
				offset_to_points: header.offset_to_point_data,
				num_vlrs: header.number_of_variable_length_records,
				point_format_id: header.point_data_record_format,
				point_size: header.point_data_record_length,
				num_points: header.number_of_point_records as u64,
				header_size: header.header_size,
			},
		)
		.unwrap();

		let total = header.number_of_point_records as u64;
		let point_length = header.point_data_record_length as usize;
		let scale = Vector::new([
			header.x_scale_factor,
			header.y_scale_factor,
			header.z_scale_factor,
		]);
		let offset = Vector::new([header.x_offset, header.y_offset, header.z_offset]);
		let min = Vector::new([header.min_x, header.min_z, -header.max_y]);
		let max = Vector::new([header.max_x, header.max_z, -header.min_y]);
		let center = (min + max) / 2.0;

		let chunks = ChunkTable::read_from(&mut file, &vlr).unwrap();

		match vlr.items().first().unwrap().version() {
			3 | 4 => {},
			_ => return Err(Error::WrongLazVersion),
		}

		Ok(Self {
			file,
			chunks,
			chunk_index: 0,
			vlr,
			remaining: total as usize,
			point_length,
			scale,
			offset,
			center,
			path: path.to_owned(),

			min: (min - center).map(|x| x as f32),
			max: (max - center).map(|x| x as f32),
		})
	}

	pub fn read(&mut self, cb: impl Fn(Chunk) + std::marker::Sync) {
		let mut start = self.file.stream_position().unwrap();
		let chunks = self
			.chunks
			.into_iter()
			.map(|x| {
				let s = start;
				start += x.byte_count;

				let l = (x.point_count as usize).min(self.remaining);
				self.remaining -= l;
				(s, l)
			})
			.collect::<Vec<_>>();

		chunks.into_par_iter().for_each_init(
			|| std::fs::File::open(&self.path).unwrap(),
			|file, (s, l)| {
				file.seek(SeekFrom::Start(s)).unwrap();

				let mut decompress = LayeredPointRecordDecompressor::new(file);
				decompress.set_fields_from(self.vlr.items()).unwrap();

				let mut slice = vec![0; l * self.point_length];
				decompress.decompress_many(&mut slice).unwrap();

				let chunk = Chunk {
					slice,
					current: 0,
					point_length: self.point_length,

					offset: self.offset,
					scale: self.scale,
					center: self.center,
				};

				cb(chunk);
			},
		);
	}
}

pub struct Chunk {
	slice: Vec<u8>,
	current: usize,
	point_length: usize,
	offset: Vector<3, f64>,
	scale: Vector<3, f64>,
	center: Vector<3, f64>,
}

impl Chunk {
	pub fn length(&self) -> usize {
		self.slice.len() / self.point_length
	}
}

impl Iterator for Chunk {
	type Item = Vector<3, f32>;

	fn next(&mut self) -> Option<Vector<3, f32>> {
		if self.current >= self.slice.len() {
			return None;
		}
		let slice = &self.slice[self.current..(self.current + 12)];
		let x = i32::from_le_bytes(slice[0..4].try_into().unwrap());
		let y = i32::from_le_bytes(slice[4..8].try_into().unwrap());
		let z = i32::from_le_bytes(slice[8..12].try_into().unwrap());
		self.current += self.point_length;

		let v = Vector::new([
			self.offset[X] + x as f64 * self.scale[X],
			self.offset[Y] + y as f64 * self.scale[Y],
			self.offset[Z] + z as f64 * self.scale[Z],
		]);
		Some((Vector::new([v[X], v[Z], -v[Y]]) - self.center).map(|x| x as f32))
	}
}
