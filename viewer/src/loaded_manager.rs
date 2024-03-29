use std::collections::{HashMap, HashSet};
use std::hint::spin_loop;
use std::path::PathBuf;
use std::sync::Arc;

use crate::reader::Reader;
use crate::State;

pub struct LoadedManager {
	available: HashMap<usize, render::PointCloud>,
	requested: HashSet<usize>,
	sender: crossbeam::channel::Sender<WorkerTask>,
	reciever: crossbeam::channel::Receiver<Response>,

	update_senders: Vec<crossbeam::channel::Sender<WorkerUpdate>>,

	property_default: render::PointCloudProperty,
	property_available: HashMap<usize, render::PointCloudProperty>,
	property_requested: HashSet<usize>,
	property_index: usize,
}

enum WorkerTask {
	PointCloud(usize),
	Property(usize),
}

enum Response {
	PointCloud(usize, render::PointCloud),
	Property(usize, render::PointCloudProperty, usize),
	FailedPointCloud(usize),
	FailedProperty(usize),
}

enum WorkerUpdate {
	ChangeProperty(String, usize),
}

impl LoadedManager {
	pub fn new(state: Arc<State>, path: Option<PathBuf>, property: &str) -> Self {
		let (index_tx, index_rx) = crossbeam::channel::bounded(512);
		let (pc_tx, pc_rx) = crossbeam::channel::bounded(512);

		let mut update_senders = Vec::new();
		for _ in 0..2 {
			let (update_sender, update_reciever) = crossbeam::channel::bounded(2);
			update_senders.push(update_sender);
			let index_rx = index_rx.clone();
			let pc_tx = pc_tx.clone();
			let mut property_index = 0;

			let mut reader = path
				.clone()
				.map(|path| Reader::new(path, property))
				.unwrap_or(Reader::fake());
			let state = state.clone();

			std::thread::spawn(move || loop {
				for update in update_reciever.try_iter() {
					match update {
						WorkerUpdate::ChangeProperty(property, index) => {
							reader.change_property(&property);
							property_index = index;
						},
					}
				}

				let task = match index_rx.try_recv() {
					Ok(v) => v,
					Err(crossbeam::channel::TryRecvError::Disconnected) => return,
					Err(crossbeam::channel::TryRecvError::Empty) => {
						spin_loop();
						continue;
					},
				};
				match task {
					WorkerTask::PointCloud(index) => {
						if let Some(pc) = load_pointcloud(&state, &mut reader, index) {
							let _ = pc_tx.send(Response::PointCloud(index, pc));
						} else {
							let _ = pc_tx.send(Response::FailedPointCloud(index));
						};
					},
					WorkerTask::Property(index) => {
						if let Some(property) = load_property(&state, &mut reader, index) {
							let _ = pc_tx.send(Response::Property(index, property, property_index));
						} else {
							let _ = pc_tx.send(Response::FailedProperty(index));
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

			property_index: 0,
			property_default: render::PointCloudProperty::new_empty(&state),
			property_available: HashMap::new(),
			property_requested: HashSet::new(),
		}
	}

	pub fn change_property(&mut self, name: &str) {
		self.property_index = self.property_index.overflowing_add(1).0;
		for sender in &self.update_senders {
			sender
				.send(WorkerUpdate::ChangeProperty(
					name.into(),
					self.property_index,
				))
				.unwrap();
		}
		self.property_available.clear();
		self.property_requested.clear();
	}

	pub fn request(&mut self, index: usize) {
		if !self.requested.contains(&index) && self.sender.try_send(WorkerTask::PointCloud(index)).is_ok() {
			self.requested.insert(index);
		}
		if !self.property_requested.contains(&index) && self.sender.try_send(WorkerTask::Property(index)).is_ok() {
			self.property_requested.insert(index);
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

	pub fn update(&mut self) -> bool {
		if self.reciever.is_empty() {
			return false;
		}
		for response in self.reciever.try_iter() {
			match response {
				Response::PointCloud(index, data) => {
					if self.requested.contains(&index) {
						self.available.insert(index, data);
					}
				},
				Response::Property(index, property, property_index) => {
					if property_index != self.property_index {
						continue;
					}
					if self.property_requested.contains(&index) {
						self.property_available.insert(index, property);
					}
				},
				Response::FailedPointCloud(index) => {
					self.requested.remove(&index);
					self.available.remove(&index);
				},
				Response::FailedProperty(index) => {
					self.property_requested.remove(&index);
					self.property_available.remove(&index);
				},
			}
		}
		true
	}
}

fn load_pointcloud(state: &State, reader: &mut Reader, index: usize) -> Option<render::PointCloud> {
	let data = reader.get_points(index);
	if data.is_empty() {
		return None;
	}
	Some(render::PointCloud::new(state, &data))
}

fn load_property(state: &State, reader: &mut Reader, index: usize) -> Option<render::PointCloudProperty> {
	let data = reader.get_property(index);
	if data.is_empty() {
		return None;
	}
	Some(render::PointCloudProperty::new(state, &data))
}
