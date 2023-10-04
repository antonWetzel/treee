use std::collections::{HashMap, HashSet};
use std::io::Read;

use crate::point_cloud::PointCloud;
use render::gpu;

pub struct LoadedManager {
	available: HashMap<usize, PointCloud>,
	requested: HashSet<usize>,
	sender: std::sync::mpsc::Sender<Action>,
	reciever: std::sync::mpsc::Receiver<(usize, PointCloud)>,
}

enum Action {
	Load(usize),
	Unload(usize),
}

impl LoadedManager {
	pub fn new<'a>(state: &'a render::State, path: String, scope: &'a std::thread::Scope<'a, '_>) -> Self {
		let (index_tx, index_rx) = std::sync::mpsc::channel();
		let (pc_tx, pc_rx) = std::sync::mpsc::channel();

		scope.spawn(move || {
			let mut work = HashSet::new();
			loop {
				loop {
					let action = match index_rx.try_recv() {
						Ok(v) => v,
						Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
						Err(std::sync::mpsc::TryRecvError::Empty) => break,
					};
					match action {
						Action::Load(v) => work.insert(v),
						Action::Unload(v) => work.remove(&v),
					};
				}
				let mut removed = None;
				if work.len() > 0 {
					println!("todo: {}", work.len());
				}
				for &index in work.iter() {
					let path: String = format!("{}/data/{}.data", path, index);
					let pc = match load(path, state) {
						Some(pc) => pc,
						None => continue,
					};
					let _ = pc_tx.send((index, pc));
					removed = Some(index);
					break;
				}
				if let Some(removed) = removed {
					work.remove(&removed);
				}
			}
		});

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

	pub fn render<'a, 'b: 'a>(&'b self, index: usize, render_pass: &mut render::RenderPass<'a>) {
		let pc = self.available.get(&index);
		match pc {
			None => {},
			Some(pc) => pc.gpu.render(render_pass),
		}
	}

	pub fn update(&mut self) {
		for (index, pc) in self.reciever.try_iter() {
			if self.requested.contains(&index) {
				self.available.insert(index, pc);
			}
		}
	}
}

fn load<T: Into<String>>(path: T, state: &render::State) -> Option<PointCloud> {
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
	let mut data = Vec::with_capacity(size);
	unsafe {
		data.set_len(size);
		let view = std::slice::from_raw_parts_mut(
			data.as_mut_ptr() as *mut u8,
			std::mem::size_of::<render::Point>() * size,
		);
		file.read_exact(view).ok()?;
	};
	let gpu = gpu::PointCloud::new(state, &data);
	Some(PointCloud { gpu })
}
