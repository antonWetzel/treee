use laz::{
	las::file::{read_vlrs_and_get_laszip_vlr, QuickHeader},
	laszip::ChunkTable,
	record::{LayeredPointRecordDecompressor, RecordDecompressor, SequentialPointRecordDecompressor},
	LazVlr,
};
use math::{Vector, X, Y, Z};
use rayon::prelude::*;
use std::{
	io::{Read, Seek, SeekFrom},
	path::{Path, PathBuf},
};

use crate::Error;

pub struct Laz {
	pub total: usize,
	vlr: LazVlr,
	chunks: Vec<(u64, usize)>,
	point_length: usize,
	center: Vector<3, f64>,
	offset: Vector<3, f64>,
	scale: Vector<3, f64>,

	path: PathBuf,

	pub min: Vector<3, f32>,
	pub max: Vector<3, f32>,
}

impl Laz {
	pub fn new(path: &Path) -> Result<Self, Error> {
		let mut file = std::fs::File::open(path)?;

		let header = Header::new(&mut file)?;

		file.seek(SeekFrom::Start(header.header_size as u64))?;
		let vlr = read_vlrs_and_get_laszip_vlr(
			&mut file,
			&QuickHeader {
				major: header.version_major,
				minor: header.version_minor,
				offset_to_points: header.offset_to_point_data,
				num_vlrs: header.number_of_variable_length_records,
				point_format_id: header.point_data_record_format,
				point_size: header.point_data_record_length,
				num_points: header.number_of_point_records,
				header_size: header.header_size,
			},
		)
		.ok_or(Error::CorruptFile)?;

		let total = header.number_of_point_records as usize;

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

		let chunks = ChunkTable::read_from(&mut file, &vlr)?;

		let mut start = file.stream_position()?;
		let mut remaining = total;
		let chunks = chunks
			.into_iter()
			.map(|x| {
				let s = start;
				start += x.byte_count;

				let l = (x.point_count as usize).min(remaining);
				remaining -= l;
				(s, l)
			})
			.collect::<Vec<_>>();

		Ok(Self {
			chunks,
			total,
			vlr,
			point_length,
			scale,
			offset,
			center,
			path: path.to_owned(),

			min: (min - center).map(|x| x as f32),
			max: (max - center).map(|x| x as f32),
		})
	}

	pub fn read(self, cb: impl Fn(Chunk) + std::marker::Sync) -> Result<(), Error> {
		self.chunks
			.into_par_iter()
			.map_init(
				|| std::fs::File::open(&self.path).unwrap(),
				|file, (s, l)| {
					file.seek(SeekFrom::Start(s))?;

					let mut slice = vec![0; l * self.point_length];
					match self.vlr.items().first().unwrap().version() {
						1 | 2 => {
							let mut decompress = SequentialPointRecordDecompressor::new(file);
							decompress.set_fields_from(self.vlr.items())?;
							decompress.decompress_many(&mut slice).unwrap();
						},
						3 | 4 => {
							let mut decompress = LayeredPointRecordDecompressor::new(file);
							decompress.set_fields_from(self.vlr.items())?;
							decompress.decompress_many(&mut slice).unwrap();
						},
						v => unimplemented!("Laz version {}", v),
					}

					let chunk = Chunk {
						slice,
						current: 0,
						point_length: self.point_length,

						offset: self.offset,
						scale: self.scale,
						center: self.center,
					};

					cb(chunk);
					Ok(())
				},
			)
			.find_any(|r| r.is_err())
			.unwrap_or(Ok(()))
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

#[repr(C, packed)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Default, Debug)]
struct Header {
	signature: [u8; 4],
	source_id: u16,
	global_encoding: u16,
	guid_1: u32,
	guid_2: u16,
	guid_3: u16,
	guid_4: [u8; 8],
	version_major: u8,
	version_minor: u8,
	system_identifier: [u8; 32],
	generating_software: [u8; 32],
	creation_day: u16,
	creation_year: u16,
	header_size: u16,
	offset_to_point_data: u32,
	number_of_variable_length_records: u32,
	point_data_record_format: u8,
	point_data_record_length: u16,
	legacy_point_amount: u32,
	legacy_point_amount_return: [u32; 5],
	x_scale_factor: f64,
	y_scale_factor: f64,
	z_scale_factor: f64,
	x_offset: f64,
	y_offset: f64,
	z_offset: f64,
	max_x: f64,
	min_x: f64,
	max_y: f64,
	min_y: f64,
	max_z: f64,
	min_z: f64,
	waveform_offset: u64,
	first_vlr_offset: u64,
	number_of_extended_variable_length_records: u32,
	number_of_point_records: u64,
	point_amount_return: [u64; 15],
}

static_assertions::assert_cfg!(target_endian = "little");

impl Header {
	pub fn new<R: Seek + Read>(mut source: R) -> Result<Self, Error> {
		let mut header = [Self::default()];
		source
			.read_exact(bytemuck::cast_slice_mut(&mut header))
			.unwrap();
		let mut header = header[0];
		if header.legacy_point_amount != 0 {
			header.number_of_point_records = header.legacy_point_amount as u64;
		}
		if &header.signature != b"LASF" {
			return Err(Error::CorruptFile);
		}

		Ok(header)
	}
}
