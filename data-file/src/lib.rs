use std::{
	fs::File,
	io::{Read, Seek, Write},
	path::Path,
};

pub struct DataFile<T>
where
	T: Copy + bytemuck::Pod,
{
	file: File,
	phantom: std::marker::PhantomData<T>,
}

impl<T> DataFile<T>
where
	T: Copy + bytemuck::Pod,
{
	pub fn new(size: usize, path: impl AsRef<Path>) -> Self {
		let file = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.open(path.as_ref())
			.unwrap();
		file.set_len((size * 2 * std::mem::size_of::<u64>()) as u64)
			.unwrap();
		Self { file, phantom: std::marker::PhantomData }
	}

	pub fn open(path: impl AsRef<Path>) -> Self {
		Self {
			file: std::fs::OpenOptions::new()
				.read(true)
				.open(path.as_ref())
				.unwrap(),
			phantom: std::marker::PhantomData,
		}
	}

	pub fn fake() -> Self {
		let mut file = tempfile::tempfile().unwrap();
		file.write_all(bytemuck::cast_slice(&[0u64, 0u64])).unwrap();
		Self { file, phantom: std::marker::PhantomData }
	}

	pub fn save(&mut self, idx: usize, data: &[T]) {
		self.file.seek(std::io::SeekFrom::End(0)).unwrap();
		let pos = [self.file.stream_position().unwrap(), data.len() as u64];
		self.file.write_all(bytemuck::cast_slice(data)).unwrap();
		self.file
			.seek(std::io::SeekFrom::Start(
				(idx * 2 * std::mem::size_of::<u64>()) as u64,
			))
			.unwrap();
		self.file.write_all(bytemuck::cast_slice(&pos)).unwrap();
	}

	pub fn read(&mut self, idx: usize) -> Vec<T> {
		let mut pos = [0u64, 0u64];
		self.file
			.seek(std::io::SeekFrom::Start(
				(idx * 2 * std::mem::size_of::<u64>()) as u64,
			))
			.unwrap();
		self.file
			.read_exact(bytemuck::cast_slice_mut(&mut pos))
			.unwrap();
		self.file.seek(std::io::SeekFrom::Start(pos[0])).unwrap();
		let mut buffer = vec![T::zeroed(); pos[1] as usize];
		self.file
			.read_exact(bytemuck::cast_slice_mut(&mut buffer))
			.unwrap();
		buffer
	}

	pub fn sizes(&mut self, size: usize) -> Vec<[u64; 2]> {
		let mut buffer = vec![[0, 0]; size];
		self.file.seek(std::io::SeekFrom::Start(0)).unwrap();
		self.file
			.read_exact(bytemuck::cast_slice_mut(&mut buffer))
			.unwrap();
		buffer
	}
}
