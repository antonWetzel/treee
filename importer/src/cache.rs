use std::{
	collections::HashMap,
	fs::File,
	io::{Read, Seek, Write},
	mem::MaybeUninit,
};

use math::Vector;

pub struct Cache {
	active: HashMap<usize, (Vec<Vector<3, f32>>, usize)>,
	stored: HashMap<usize, File>,
	current: usize,
}

#[derive(Debug)]
pub struct CacheEntry {
	file: Option<File>,
	active: Vec<Vector<3, f32>>,
}

impl Cache {
	const MAX: usize = 64;

	pub fn new() -> Self {
		Self {
			active: HashMap::new(),
			stored: HashMap::new(),
			current: 0,
		}
	}

	pub fn add_point(&mut self, index: usize, point: Vector<3, f32>) {
		self.current += 1;
		match self.active.get_mut(&index) {
			None => {},
			Some(entry) => {
				entry.0.push(point);
				entry.1 = self.current;
				return;
			},
		}
		self.evict();
		self.active.insert(index, (vec![point], self.current));
	}

	fn evict(&mut self) {
		if self.active.len() < Self::MAX {
			return;
		}
		let mut oldest_index = 0;
		let mut oldest_value = usize::MAX;
		for (index, entry) in &self.active {
			if entry.1 < oldest_value {
				oldest_index = *index;
				oldest_value = entry.1;
			}
		}

		fn write_to(file: &mut File, data: Vec<Vector<3, f32>>) {
			unsafe {
				let view = std::slice::from_raw_parts(
					data.as_ptr() as *const u8,
					std::mem::size_of::<Vector<3, f32>>() * data.len(),
				);
				file.write_all(view).unwrap();
			}
		}
		let entry = self.active.remove(&oldest_index).unwrap().0;
		match self.stored.get_mut(&oldest_index) {
			None => {
				let mut file = tempfile::tempfile().unwrap();
				write_to(&mut file, entry);
				self.stored.insert(oldest_index, file);
			},
			Some(file) => write_to(file, entry),
		}
	}

	pub fn read(&mut self, index: usize) -> CacheEntry {
		CacheEntry {
			file: self.stored.remove(&index),
			active: self.active.remove(&index).map(|v| v.0).unwrap_or_default(),
		}
	}
}

impl CacheEntry {
	pub fn read(mut self) -> Vec<Vector<3, f32>> {
		if let Some(mut file) = self.file {
			unsafe {
				let mut data = Vec::<MaybeUninit<Vector<3, f32>>>::new();
				let l = file.metadata().unwrap().len() as usize / std::mem::size_of::<Vector<3, f32>>();
				data.reserve(l + self.active.len());
				data.set_len(l);
				let view = std::slice::from_raw_parts_mut(
					data.as_mut_ptr() as *mut u8,
					std::mem::size_of::<Vector<3, f32>>() * l,
				);
				file.seek(std::io::SeekFrom::Start(0)).unwrap();
				file.read_exact(view).unwrap();
				let mut data = std::mem::transmute::<_, Vec<Vector<3, f32>>>(data);
				data.append(&mut self.active);
				data
			}
		} else {
			self.active
		}
	}
}
