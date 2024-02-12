use laz::{
	las::file::{read_vlrs_and_get_laszip_vlr, QuickHeader},
	ParLasZipDecompressor,
};
use math::{Vector, X, Y, Z};
use std::{
	fs::File,
	io::{Read, Seek, SeekFrom},
	path::Path,
};

pub struct Laz {
	decompressor: ParLasZipDecompressor<File>,
	chunk_size: usize,
	pub remaining: usize,
	point_length: usize,
	center: Vector<3, f64>,
	offset: Vector<3, f64>,
	scale: Vector<3, f64>,

	pub min: Vector<3, f32>,
	pub max: Vector<3, f32>,
}

impl Laz {
	pub fn new(path: &Path) -> Result<Self, std::io::Error> {
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

		let total = header.number_of_point_records as usize;
		let chunk_size = vlr.chunk_size() as usize;
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

		file.seek(SeekFrom::Start(header.offset_to_point_data as u64))
			.unwrap();
		let decompressor = laz::ParLasZipDecompressor::new(file, vlr).unwrap();

		Ok(Self {
			decompressor,
			chunk_size,
			remaining: total,
			point_length,
			scale,
			offset,
			center,

			min: (min - center).map(|x| x as f32),
			max: (max - center).map(|x| x as f32),
		})
	}
}

impl Iterator for Laz {
	type Item = Chunk;

	fn next(&mut self) -> Option<Chunk> {
		let size = self.chunk_size * rayon::current_num_threads() * 16;
		let size = if self.remaining < size {
			if self.remaining == 0 {
				return None;
			}
			self.remaining
		} else {
			size
		};
		self.remaining -= size;
		let mut slice = bytemuck::zeroed_vec(size * self.point_length);
		self.decompressor.decompress_many(&mut slice).unwrap();

		Some(Chunk {
			slice,
			current: 0,
			point_length: self.point_length,

			offset: self.offset,
			scale: self.scale,
			center: self.center,
		})
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
