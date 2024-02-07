use std::{
	collections::HashMap,
	fs::File,
	io::{Read, Seek, Write},
	mem::MaybeUninit,
	ops::Not,
};

pub struct Cache {
	active: HashMap<usize, (Vec<u8>, usize)>,
	stored: HashMap<usize, (File, usize)>,
	current: usize,
	max_values: usize,
	entry_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheIndex<T>(usize, std::marker::PhantomData<T>);

#[derive(Debug)]
pub struct CacheEntry<T> {
	file: Option<File>,
	active: Vec<T>,
	length: usize,
}

impl Cache {
	pub fn new(max_values: usize) -> Self {
		Self {
			active: HashMap::new(),
			stored: HashMap::new(),
			current: 0,
			max_values,
			entry_index: 0,
		}
	}

	pub fn new_entry<T>(&mut self) -> CacheIndex<T> {
		self.entry_index += 1;
		CacheIndex(self.entry_index, std::marker::PhantomData)
	}

	pub fn add_value<T>(&mut self, index: &CacheIndex<T>, point: T) {
		match self.active.get_mut(&index.0) {
			None => {
				let vec = vec![point];
				self.current += vec.capacity() * std::mem::size_of::<T>();
				self.active.insert(
					index.0,
					(
						unsafe { std::mem::transmute(vec) },
						std::mem::size_of::<T>(),
					),
				);
			},
			Some(entry) => {
				let data = unsafe { std::mem::transmute::<_, &mut Vec<T>>(&mut entry.0) };
				let c = data.capacity();
				data.push(point);
				self.current += entry.1 * (data.capacity() - c);
			},
		}

		if self.current >= self.max_values {
			self.evict();
		}
	}

	fn evict(&mut self) {
		let Some((&key, _)) = self
			.active
			.iter()
			.max_by_key(|(_, entry)| entry.0.len() * entry.1)
		else {
			return;
		};

		fn write_to(file: &mut File, data: Vec<u8>, size: usize) {
			unsafe {
				let view = std::slice::from_raw_parts(data.as_ptr(), size * data.len());
				file.write_all(view).unwrap();
			}
		}
		let entry = self.active.remove(&key).unwrap();
		self.current -= entry.0.capacity() * entry.1;

		match self.stored.get_mut(&key) {
			None => {
				let mut file = tempfile::tempfile().unwrap();
				let l = entry.0.len();
				write_to(&mut file, entry.0, entry.1);
				self.stored.insert(key, (file, l));
			},
			Some((file, length)) => {
				*length += entry.0.len();
				write_to(file, entry.0, entry.1);
			},
		}
	}

	pub fn read<T>(&mut self, index: CacheIndex<T>) -> CacheEntry<T> {
		let active = self.active.remove(&index.0).unwrap_or_default();
		let active = unsafe { std::mem::transmute::<_, Vec<T>>(active.0) };
		self.current -= active.capacity() * std::mem::size_of::<T>();
		let (file, length) = if let Some((file, length)) = self.stored.remove(&index.0) {
			(Some(file), length + active.len())
		} else {
			(None, active.len())
		};
		CacheEntry { length, file, active }
	}

	pub fn size<T>(&mut self, index: &CacheIndex<T>) -> usize {
		self.active
			.get(&index.0)
			.map(|active| active.0.len())
			.unwrap_or_default()
			+ self
				.stored
				.get(&index.0)
				.map(|&(_, l)| l)
				.unwrap_or_default()
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
