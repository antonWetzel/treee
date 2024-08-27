use laz::{
	las::file::{read_vlrs_and_get_laszip_vlr, QuickHeader},
	laszip::ChunkTable,
	record::{
		LayeredPointRecordDecompressor, RecordDecompressor, SequentialPointRecordDecompressor,
	},
	LazVlr,
};
use nalgebra as na;

use rayon::prelude::*;
use std::io::{Read, Seek, SeekFrom};

use crate::{environment, Error};

pub struct Laz {
	vlr: Option<LazVlr>,
	chunks: Vec<(u64, usize)>,
	point_length: usize,
	center: na::Point3<f64>,
	offset: na::Point3<f64>,
	scale: na::Point3<f64>,

	source: environment::Source,

	pub min: na::Point3<f32>,
	pub max: na::Point3<f32>,
	pub world_offset: na::Point3<f64>,
}

impl Laz {
	pub fn new(
		source: environment::Source,
		center: Option<na::Point3<f64>>,
	) -> Result<Self, Error> {
		let mut reader = source.reader();

		let header = Header::new(&mut reader)?;

		#[allow(unused_mut)]
		let mut total = header.number_of_point_records as usize;
		log::info!("Loading Pointcloud with {} points", total);

		#[cfg(target_arch = "wasm32")]
		{
			use wasm_bindgen::prelude::*;

			const MAX_NUMBER_POINTS: usize = 15_000_000;

			#[wasm_bindgen]
			extern "C" {
				fn alert(s: &str);
			}

			if total > MAX_NUMBER_POINTS {
				unsafe {
					alert(&format!(
						"Point cloud to large, only the first {} points are used!",
						MAX_NUMBER_POINTS
					))
				};
				total = MAX_NUMBER_POINTS;
			}
		}

		let point_length = header.point_data_record_length as usize;
		let scale = na::Point3::new(
			header.x_scale_factor,
			header.y_scale_factor,
			header.z_scale_factor,
		);
		let offset = na::Point3::new(header.x_offset, header.y_offset, header.z_offset);
		let min = na::Point3::new(header.min_x, header.min_z, -header.max_y);
		let max = na::Point3::new(header.max_x, header.max_z, -header.min_y);
		let center = center.unwrap_or(na::center(&min, &max));

		reader.seek(SeekFrom::Start(header.header_size as u64))?;
		let (chunks, vlr) =
			if let Some(vlr) = read_vlrs_and_get_laszip_vlr(&mut reader, &header.quick_header()) {
				let chunks = ChunkTable::read_from(&mut reader, &vlr)?;
				let mut start = reader.stream_position()?;
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

				(chunks, Some(vlr))
			} else {
				let mut chunks = Vec::new();
				let mut start = header.offset_to_point_data as u64;
				const DEFAULT_CHUNK_SIZE: usize = 50_000;
				for _ in 0..(total / DEFAULT_CHUNK_SIZE) {
					chunks.push((start, DEFAULT_CHUNK_SIZE));
					start += (DEFAULT_CHUNK_SIZE * point_length) as u64;
				}
				let rem = total % DEFAULT_CHUNK_SIZE;
				if rem != 0 {
					chunks.push((start, rem));
				}
				(chunks, None)
			};
		drop(reader);

		Ok(Self {
			chunks,
			vlr,
			point_length,
			scale,
			offset,
			center,
			source,

			min: (min - center).map(|x| x as f32).into(),
			max: (max - center).map(|x| x as f32).into(),
			world_offset: center,
		})
	}

	pub fn total(&self) -> usize {
		self.chunks.len()
	}

	pub fn read(
		self,
		cb: impl Fn(Chunk) -> Result<(), Error> + std::marker::Sync,
	) -> Result<(), Error> {
		self.chunks
			.into_iter()
			.par_bridge()
			.map_init(
				|| self.source.reader(),
				|file, (s, l)| {
					let slice = if l == 0 {
						Vec::new()
					} else {
						file.seek(SeekFrom::Start(s))?;

						let mut slice = vec![0; l * self.point_length];
						if let Some(vlr) = &self.vlr {
							match vlr.items().first().unwrap().version() {
								1 | 2 => {
									let mut decompress =
										SequentialPointRecordDecompressor::new(file);
									decompress.set_fields_from(vlr.items())?;
									decompress.decompress_many(&mut slice).unwrap();
								},
								3 | 4 => {
									let mut decompress = LayeredPointRecordDecompressor::new(file);
									decompress.set_fields_from(vlr.items())?;
									decompress.decompress_many(&mut slice).unwrap();
								},
								v => unimplemented!("Laz version {}", v),
							}
						} else {
							file.read_exact(&mut slice)?;
						}
						slice
					};
					let chunk = Chunk {
						slice,
						current: 0,
						point_length: self.point_length,

						offset: self.offset,
						scale: self.scale,
						center: self.center,
					};
					cb(chunk)
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
	offset: na::Point3<f64>,
	scale: na::Point3<f64>,
	center: na::Point3<f64>,
}

impl Chunk {
	pub fn read(mut self) -> Vec<na::Point3<f32>> {
		let amount = self.slice.len() / self.point_length;
		let mut res = Vec::with_capacity(amount);
		for _ in 0..amount {
			res.push(self.next_point());
		}
		res
	}

	fn next_point(&mut self) -> na::Point3<f32> {
		let slice = &self.slice[self.current..(self.current + 12)];
		let x = i32::from_le_bytes(slice[0..4].try_into().unwrap());
		let y = i32::from_le_bytes(slice[4..8].try_into().unwrap());
		let z = i32::from_le_bytes(slice[8..12].try_into().unwrap());
		self.current += self.point_length;
		let v = self.offset
			+ na::vector![x as f64, y as f64, z as f64].zip_map(&self.scale.coords, |a, b| a * b);
		(na::vector![v.x, v.z, -v.y] - self.center.coords)
			.map(|x| x as f32)
			.into()
	}
}

impl Iterator for Chunk {
	type Item = na::Point3<f32>;

	fn next(&mut self) -> Option<na::Point3<f32>> {
		if self.current >= self.slice.len() {
			return None;
		}
		let p = self.next_point();
		Some(p)
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

	pub fn quick_header(&self) -> QuickHeader {
		QuickHeader {
			major: self.version_major,
			minor: self.version_minor,
			offset_to_points: self.offset_to_point_data,
			num_vlrs: self.number_of_variable_length_records,
			point_format_id: self.point_data_record_format,
			point_size: self.point_data_record_length,
			num_points: self.number_of_point_records,
			header_size: self.header_size,
		}
	}
}
