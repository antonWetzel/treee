use std::collections::{HashMap, HashSet};
use std::hint::spin_loop;
use std::io::Read;
use std::mem::MaybeUninit;
use std::path::{Path, PathBuf};

use crate::State;

pub struct LoadedManager {
	available: HashMap<usize, render::PointCloud>,
	requested: HashSet<usize>,
	sender: crossbeam_channel::Sender<WorkerTask>,
	reciever: crossbeam_channel::Receiver<Response>,

	update_senders: Vec<crossbeam_channel::Sender<WorkerUpdate>>,

	property_default: render::PointCloudProperty,
	property_available: HashMap<usize, render::PointCloudProperty>,
	property_requested: HashSet<usize>,
}

enum WorkerTask {
	PointCloud(usize),
	Property(usize),
}
enum Response {
	PointCloud(usize, render::PointCloud),
	Property(usize, render::PointCloudProperty),
	RedoPointCloud(usize),
	RedoProperty(usize),
}

enum WorkerUpdate {
	ChangeProperty(String),
}

impl LoadedManager {
	pub fn new(state: &'static State, mut path: PathBuf, property: &str) -> Self {
		let (index_tx, index_rx) = crossbeam_channel::bounded(512);
		let (pc_tx, pc_rx) = crossbeam_channel::bounded(512);
		path.push("data");
		let mut property_path = path.clone();
		property_path.push(property);
		path.push("0.data");
		property_path.push("0.data");

		let mut update_senders = Vec::new();
		for _ in 0..2 {
			let (update_sender, update_reciever) = crossbeam_channel::bounded(2);
			update_senders.push(update_sender);
			let index_rx = index_rx.clone();
			let mut path = path.clone();
			let mut property_path = property_path.clone();
			let pc_tx = pc_tx.clone();
			std::thread::spawn(move || loop {
				for update in update_reciever.try_iter() {
					match update {
						WorkerUpdate::ChangeProperty(name) => {
							property_path = path.parent().unwrap().to_owned();
							property_path.push(name);
							path.push("0.data");
						},
					}
				}

				let task = match index_rx.try_recv() {
					Ok(v) => v,
					Err(crossbeam_channel::TryRecvError::Disconnected) => return,
					Err(crossbeam_channel::TryRecvError::Empty) => {
						spin_loop();
						continue;
					},
				};
				match task {
					WorkerTask::PointCloud(index) => {
						path.set_file_name(format!("{}.data", index));
						if let Some(pc) = load_pointcloud(&path, state) {
							let _ = pc_tx.send(Response::PointCloud(index, pc));
						} else {
							let _ = pc_tx.send(Response::RedoPointCloud(index));
						};
					},
					WorkerTask::Property(index) => {
						property_path.set_file_name(format!("{}.data", index));
						if let Some(property) = load_property(&property_path, state) {
							let _ = pc_tx.send(Response::Property(index, property));
						} else {
							let _ = pc_tx.send(Response::RedoProperty(index));
						};
					},
				}
			});
		}

		Self {
			available: HashMap::new(),
			requested: HashSet::new(),
			sender: index_tx,
			reciever: pc_rx,
			update_senders,

			property_default: render::PointCloudProperty::new_empty(state),
			property_available: HashMap::new(),
			property_requested: HashSet::new(),
		}
	}

	pub fn change_property(&mut self, name: &str) {
		for sender in &self.update_senders {
			sender
				.send(WorkerUpdate::ChangeProperty(String::from(name)))
				.unwrap();
		}
		//todo: clear properties
	}

	pub fn request(&mut self, index: usize) {
		if !self.requested.contains(&index) {
			if self.sender.try_send(WorkerTask::PointCloud(index)).is_ok() {
				self.requested.insert(index);
			}
		}
		if !self.property_requested.contains(&index) {
			if self.sender.try_send(WorkerTask::Property(index)).is_ok() {
				self.property_requested.insert(index);
			}
		}
	}

	pub fn unload(&mut self, index: usize) {
		self.available.remove(&index);
		self.requested.remove(&index);
		self.property_available.remove(&index);
		self.property_requested.remove(&index);
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
			Some(pc) => pc.render(
				point_cloud_pass,
				self.property_available
					.get(&index)
					.unwrap_or(&self.property_default),
			),
		}
	}

	pub fn update(&mut self) -> usize {
		for response in self.reciever.try_iter() {
			match response {
				Response::PointCloud(index, data) => {
					if self.requested.contains(&index) {
						self.available.insert(index, data);
					}
				},
				Response::Property(index, property) => {
					if self.property_requested.contains(&index) {
						self.property_available.insert(index, property);
					}
				},
				Response::RedoPointCloud(index) => self.sender.send(WorkerTask::PointCloud(index)).unwrap(),
				Response::RedoProperty(index) => self.sender.send(WorkerTask::Property(index)).unwrap(),
			}
		}
		self.sender.len()
	}
}

fn load_pointcloud(path: &Path, state: &State) -> Option<render::PointCloud> {
	let mut file = std::fs::File::open(path).ok()?;
	let file_length = file.metadata().ok()?.len() as usize;
	if file_length < 8 {
		return None;
	}
	let mut buffer = [0u8; 8];
	file.read_exact(&mut buffer).ok()?;
	let size = u64::from_le_bytes(buffer) as usize;

	if file_length != size * std::mem::size_of::<render::Point>() + 8 {
		println!("wrong length");
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
		std::mem::transmute::<_, Vec<_>>(data)
	};
	Some(render::PointCloud::new(state, &data))
}

fn load_property(path: &Path, state: &State) -> Option<render::PointCloudProperty> {
	let mut file = std::fs::File::open(path).ok()?;
	let file_length = file.metadata().ok()?.len() as usize;
	let size = file_length / std::mem::size_of::<u32>();

	let data = unsafe {
		let mut data = Vec::<MaybeUninit<u32>>::new();
		data.reserve_exact(size);
		data.set_len(size);
		let view = std::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut u8, file_length);
		file.read_exact(view).ok()?;
		std::mem::transmute::<_, Vec<_>>(data)
	};
	Some(render::PointCloudProperty::new(state, &data))
}
