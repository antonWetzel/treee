use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::mem::MaybeUninit;

use crate::State;

pub struct LoadedManager {
	available: HashMap<usize, render::PointCloud>,
	requested: HashSet<usize>,
	sender: std::sync::mpsc::Sender<Action>,
	reciever: std::sync::mpsc::Receiver<Response>,
	workload: usize,
}

enum Action {
	Load(usize),
	Unload(usize),
}

struct Response {
	index: usize,
	data: render::PointCloud,
	workload: usize,
}

impl LoadedManager {
	pub fn new(state: &'static State, path: String) -> Self {
		let (index_tx, index_rx) = std::sync::mpsc::channel();
		let (pc_tx, pc_rx) = std::sync::mpsc::channel();

		std::thread::spawn(move || {
			let mut work = HashSet::new();
			loop {
				loop {
					let action = match index_rx.try_recv() {
						Ok(v) => v,
						Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
						Err(std::sync::mpsc::TryRecvError::Empty) => break,
					};
					match action {
						Action::Load(v) => {
							work.insert(v);
						},
						Action::Unload(v) => {
							work.remove(&v);
						},
					};
				}
				let mut removed = None;
				for &index in work.iter() {
					let path = format!("{}/data/{}.data", path, index);
					let pc = match load(path, state) {
						Some(pc) => pc,
						None => continue,
					};
					removed = Some((index, pc));
					break;
				}
				if let Some((index, data)) = removed {
					work.remove(&index);
					let _ = pc_tx.send(Response { index, data, workload: work.len() });
				}
			}
		});

		Self {
			available: HashMap::new(),
			requested: HashSet::new(),
			sender: index_tx,
			reciever: pc_rx,
			workload: 0,
		}
	}

	pub fn request(&mut self, index: usize) {
		if self.requested.contains(&index) {
			return;
		}
		self.requested.insert(index);
		self.sender.send(Action::Load(index)).unwrap();
	}

	pub fn unload(&mut self, index: usize) {
		self.available.remove(&index);
		if self.requested.remove(&index) {
			self.sender.send(Action::Unload(index)).unwrap();
		};
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
			self.workload = response.workload;
		}
		self.workload
	}
}

fn load<T: Into<String>>(path: T, state: &State) -> Option<render::PointCloud> {
	let mut file = std::fs::File::open(path.into()).ok()?;
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
