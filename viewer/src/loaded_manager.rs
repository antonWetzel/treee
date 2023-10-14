use std::collections::{HashMap, HashSet};
use std::hint::spin_loop;
use std::io::Read;
use std::mem::MaybeUninit;
use std::path::{Path, PathBuf};

use crate::State;

pub struct LoadedManager {
	available: HashMap<usize, render::PointCloud>,
	requested: HashSet<usize>,
	sender: crossbeam_channel::Sender<usize>,
	reciever: crossbeam_channel::Receiver<Response>,
}

struct Response {
	index: usize,
	data: render::PointCloud,
}

impl LoadedManager {
	pub fn new(state: &'static State, mut path: PathBuf) -> Self {
		let (index_tx, index_rx) = crossbeam_channel::bounded(512);
		let (pc_tx, pc_rx) = crossbeam_channel::bounded(512);
		path.push("data");
		path.push("0.data");
		for _ in 0..2 {
			let index_rx = index_rx.clone();
			let mut path = path.clone();
			let pc_tx = pc_tx.clone();
			std::thread::spawn(move || loop {
				let index = match index_rx.try_recv() {
					Ok(v) => v,
					Err(crossbeam_channel::TryRecvError::Disconnected) => return,
					Err(crossbeam_channel::TryRecvError::Empty) => {
						spin_loop();
						continue;
					},
				};
				path.set_file_name(format!("{}.data", index));
				let pc = match load(&path, state) {
					Some(pc) => pc,
					None => continue,
				};
				let _ = pc_tx.send(Response { index, data: pc });
			});
		}

		Self {
			available: HashMap::new(),
			requested: HashSet::new(),
			sender: index_tx,
			reciever: pc_rx,
		}
	}

	pub fn request(&mut self, index: usize) {
		if self.requested.contains(&index) {
			return;
		}
		if self.sender.try_send(index).is_ok() {
			self.requested.insert(index);
		}
	}

	pub fn unload(&mut self, index: usize) {
		self.available.remove(&index);
		self.requested.remove(&index);
	}

	pub fn exist(&self, index: usize) -> bool {
		self.available.contains_key(&index)
	}
	pub fn is_requested(&self, index: usize) -> bool {
		self.requested.contains(&index)
	}

	pub fn render<'a>(&'a self, index: usize, point_cloud_pass: &mut render::PointCloudPass<'a>) {
		let pc = self.available.get(&index);
		match pc {
			None => {},
			Some(pc) => pc.render(point_cloud_pass),
		}
	}

	pub fn update(&mut self) -> usize {
		for response in self.reciever.try_iter() {
			if self.requested.contains(&response.index) {
				self.available.insert(response.index, response.data);
			}
		}
		self.sender.len()
	}
}

fn load(path: &Path, state: &State) -> Option<render::PointCloud> {
	let mut file = std::fs::File::open(path).ok()?;
	let file_length = file.metadata().ok()?.len() as usize;
	if file_length < 8 {
		return None;
	}
	let mut buffer = [0u8; 8];
	file.read_exact(&mut buffer).ok()?;
	let size = u64::from_le_bytes(buffer);
	let size = size as usize;
	if size == 0 {
		return None;
	}
	if file_length != size * std::mem::size_of::<render::Point>() + 8 {
		return None;
	}
	let data = unsafe {
		let mut data = Vec::<MaybeUninit<render::Point>>::new();
		data.reserve_exact(size);
		data.set_len(size);
		let view = std::slice::from_raw_parts_mut(
			data.as_mut_ptr() as *mut u8,
			std::mem::size_of::<render::Point>() * size,
		);
		file.read_exact(view).ok()?;
		std::mem::transmute(data)
	};
	Some(render::PointCloud::new(state, &data))
}
