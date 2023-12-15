use std::{
	collections::HashMap,
	fs::File,
	io::{Read, Seek, Write},
	mem::MaybeUninit,
	ops::Not,
};

pub struct Cache<T> {
	active: HashMap<usize, (Vec<T>, usize)>,
	stored: HashMap<usize, (File, usize)>,
	current: usize,
	max_active: usize,
}

#[derive(Debug)]
pub struct CacheIndex(usize);

#[derive(Debug)]
pub struct CacheEntry<T> {
	file: Option<File>,
	active: Vec<T>,
	length: usize,
}

impl<T> Cache<T> {
	pub fn new(max_active: usize) -> Self {
		Self {
			active: HashMap::new(),
			stored: HashMap::new(),
			current: 0,
			max_active,
		}
	}

	pub fn new_entry(&mut self) -> CacheIndex {
		self.current += 1;
		CacheIndex(self.current)
	}

	pub fn add_point(&mut self, index: &CacheIndex, point: T) {
		self.current += 1;
		match self.active.get_mut(&index.0) {
			None => {},
			Some(entry) => {
				entry.0.push(point);
				entry.1 = self.current;
				return;
			},
		}
		self.evict();
		self.active.insert(index.0, (vec![point], self.current));
	}

	fn evict(&mut self) {
		if self.active.len() < self.max_active {
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

		fn write_to<T>(file: &mut File, data: Vec<T>) {
			unsafe {
				let view = std::slice::from_raw_parts(
					data.as_ptr() as *const u8,
					std::mem::size_of::<T>() * data.len(),
				);
				file.write_all(view).unwrap();
			}
		}
		let entry = self.active.remove(&oldest_index).unwrap().0;
		match self.stored.get_mut(&oldest_index) {
			None => {
				let mut file = tempfile::tempfile().unwrap();
				let l = entry.len();
				write_to(&mut file, entry);
				self.stored.insert(oldest_index, (file, l));
			},
			Some((file, length)) => {
				*length += entry.len();
				write_to(file, entry);
			},
		}
	}

	pub fn read(&mut self, index: CacheIndex) -> CacheEntry<T> {
		let active = self
			.active
			.remove(&index.0)
			.map(|v| v.0)
			.unwrap_or_default();
		let (file, length) = if let Some((file, length)) = self.stored.remove(&index.0) {
			(Some(file), length + active.len())
		} else {
			(None, active.len())
		};
		CacheEntry { length, file, active }
	}
}

impl<T> CacheEntry<T> {
	pub fn read(mut self) -> Vec<T> {
		if let Some(mut file) = self.file {
			unsafe {
				let mut data = Vec::<MaybeUninit<T>>::new();
				let l = file.metadata().unwrap().len() as usize / std::mem::size_of::<T>();
				data.reserve(l + self.active.len());
				data.set_len(l);
				let view = std::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut u8, std::mem::size_of::<T>() * l);
				file.seek(std::io::SeekFrom::Start(0)).unwrap();
				file.read_exact(view).unwrap();
				let mut data = std::mem::transmute::<_, Vec<T>>(data);
				data.append(&mut self.active);
				data
			}
		} else {
			self.active
		}
	}

	pub fn is_empty(&self) -> bool {
		self.length() == 0
	}

	pub fn active(&self) -> bool {
		self.active.is_empty().not()
	}

	pub fn length(&self) -> usize {
		self.length
	}
}
