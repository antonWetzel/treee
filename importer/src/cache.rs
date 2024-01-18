use std::{
	collections::HashMap,
	fs::File,
	io::{ Read, Seek, Write },
	mem::MaybeUninit,
	ops::Not,
};


pub struct Cache<T> {
	active: HashMap<usize, Vec<T>>,
	stored: HashMap<usize, (File, usize)>,
	current: usize,
	max_values: usize,
	entry_index: usize,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheIndex(usize);


#[derive(Debug)]
pub struct CacheEntry<T> {
	file: Option<File>,
	active: Vec<T>,
	length: usize,
}


impl<T> Cache<T> {
	pub fn new(max_values: usize) -> Self {
		Self {
			active: HashMap::new(),
			stored: HashMap::new(),
			current: 0,
			max_values,
			entry_index: 0,
		}
	}


	pub fn new_entry(&mut self) -> CacheIndex {
		self.entry_index += 1;
		CacheIndex(self.entry_index)
	}


	pub fn add_value(&mut self, index: &CacheIndex, point: T) {
		self.current += 1;
		if self.current > self.max_values {
			self.evict();
		}
		match self.active.get_mut(&index.0) {
			None => {
				self.active.insert(index.0, vec![point]);
			},
			Some(entry) => {
				entry.push(point);
			},
		}
	}


	fn evict(&mut self) {
		let Some((&key, entry)) = self.active.iter_mut().max_by_key(|(_, entry)| entry.len()) else {
			return;
		};


		fn write_to<T>(file: &mut File, data: Vec<T>) {
			unsafe {
				let view = std::slice::from_raw_parts(
					data.as_ptr() as *const u8,
					std::mem::size_of::<T>() * data.len(),
				);
				file.write_all(view).unwrap();
			}
		}


		let entry = std::mem::take(entry);
		self.current -= entry.len();

		match self.stored.get_mut(&key) {
			None => {
				let mut file = tempfile::tempfile().unwrap();
				let l = entry.len();
				write_to(&mut file, entry);
				self.stored.insert(key, (file, l));
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
			.unwrap_or_default();
		let (file, length) = if let Some((file, length)) = self.stored.remove(&index.0) {
			(Some(file), length + active.len())
		} else {
			(None, active.len())
		};
		CacheEntry { length, file, active }
	}


	pub fn size(&mut self, index: &CacheIndex) -> usize {
		self.active.get(&index.0).map(|active| active.len()).unwrap_or_default()
			+ self.stored.get(&index.0).map(|&(_, l)| l).unwrap_or_default()
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


	pub fn active(&self) -> bool {
		self.active.is_empty().not()
	}


	pub fn length(&self) -> usize {
		self.length
	}
}
