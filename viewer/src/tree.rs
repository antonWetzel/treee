use std::path::PathBuf;

use crate::loaded_manager::LoadedManager;
use crate::state::State;
use crate::{camera, lod};

use common::IndexNode;
use common::{IndexData, Project};
use math::Vector;

#[derive(Clone, Copy)]
enum LookupName {
	Warm,
	Cold,
}

impl LookupName {
	pub fn next(self) -> Self {
		match self {
			LookupName::Warm => LookupName::Cold,
			LookupName::Cold => LookupName::Warm,
		}
	}

	pub fn data(self) -> &'static [u8] {
		match self {
			LookupName::Warm => include_bytes!("../assets/grad_warm.png"),
			LookupName::Cold => include_bytes!("../assets/grad_cold.png"),
		}
	}
}

pub struct Tree {
	pub root: Node,
	pub camera: camera::Camera,
	pub loaded_manager: LoadedManager,
	pub lookup: render::Lookup,
	lookup_name: LookupName,
}

pub struct Node {
	data: Data,
	pub corner: Vector<3, f32>,
	pub size: f32,
	index: usize,
}

pub enum Data {
	Branch { children: Box<[Option<Node>; 8]> },
	Leaf(),
}

impl Node {
	pub fn new(node: &IndexNode) -> Self {
		let data = match &node.data {
			IndexData::Branch(index_children) => {
				let mut children: [_; 8] = Default::default();
				for (i, child) in index_children.iter().enumerate() {
					if let Some(child) = child {
						children[i] = Some(Node::new(child));
					}
				}
				Data::Branch { children: Box::new(children) }
			},
			IndexData::Leaf() => Data::Leaf(),
		};

		Node {
			corner: node.position,
			size: node.size,
			index: node.index as usize,
			data,
		}
	}

	pub fn render<'a>(
		&'a self,
		point_cloud_pass: &mut render::PointCloudPass<'a>,
		view_checker: lod::Checker,
		camera: &camera::Camera,
		loaded_manager: &'a LoadedManager,
	) {
		if !camera.inside_frustrum(self.corner, self.size) {
			return;
		}
		match &self.data {
			Data::Branch { children } => {
				if loaded_manager.exist(self.index)
					&& (view_checker.should_render(self.corner, self.size, camera)
						|| !Self::can_render_children(children, loaded_manager, camera))
				{
					loaded_manager.render(self.index, point_cloud_pass);
				} else {
					let view_checker = view_checker.level_down();
					for child in children.iter().flatten() {
						child.render(point_cloud_pass, view_checker, camera, loaded_manager);
					}
				}
			},
			Data::Leaf() => loaded_manager.render(self.index, point_cloud_pass),
		}
	}

	pub fn can_render_children(
		children: &[Option<Node>; 8],
		loaded_manager: &LoadedManager,
		camera: &camera::Camera,
	) -> bool {
		let mut count = 0;
		for child in children.iter().flatten() {
			if !camera.inside_frustrum(child.corner, child.size) {
				continue;
			}
			if !loaded_manager.exist(child.index) {
				count += 1;
			}
		}
		count < 1
	}

	pub fn update(&mut self, view_checker: lod::Checker, camera: &camera::Camera, loaded_manager: &mut LoadedManager) {
		if !camera.inside_moved_frustrum(self.corner, self.size, -100.0) {
			self.clear(loaded_manager);
			return;
		}
		match &mut self.data {
			Data::Branch { children } => {
				if view_checker.should_render(self.corner, self.size, camera) {
					loaded_manager.request(self.index);
					for child in children.iter_mut().flatten() {
						child.clear(loaded_manager);
					}
				} else {
					if !loaded_manager.exist(self.index) {
						loaded_manager.request(self.index);
					}
					let view_checker = view_checker.level_down();
					for child in children.iter_mut().flatten() {
						child.update(view_checker, camera, loaded_manager);
					}
				}
			},
			Data::Leaf() => {
				loaded_manager.request(self.index);
			},
		}
	}

	pub fn clear(&self, loaded_manager: &mut LoadedManager) {
		if !loaded_manager.is_requested(self.index) {
			return;
		}
		loaded_manager.unload(self.index);
		match &self.data {
			Data::Branch { children } => {
				for child in children.iter().flatten() {
					child.clear(loaded_manager);
				}
			},
			Data::Leaf() => {},
		}
	}
}

impl Tree {
	pub fn new(state: &'static State, project: &Project, path: PathBuf, aspect: f32) -> Self {
		let lookup_name = LookupName::Warm;
		Self {
			camera: camera::Camera::new(state, aspect),
			root: Node::new(&project.root),
			loaded_manager: LoadedManager::new(state, path, "height"),
			lookup_name,
			lookup: render::Lookup::new_png(state, lookup_name.data()),
		}
	}

	pub fn render<'a>(&'a self, mut point_cloud_pass: render::PointCloudPass<'a>) -> render::PointCloudPass<'a> {
		self.root.render(
			&mut point_cloud_pass,
			lod::Checker::new(&self.camera.lod),
			&self.camera,
			&self.loaded_manager,
		);
		point_cloud_pass
	}

	pub fn next_lookup(&mut self, state: &'static State) {
		self.lookup_name = self.lookup_name.next();
		self.lookup = render::Lookup::new_png(state, self.lookup_name.data());
	}
}

impl render::PointCloudEnvironment for Tree {
	fn camera(&self) -> &render::Camera3DGPU {
		&self.camera.gpu
	}
	fn lookup(&self) -> &render::Lookup {
		&self.lookup
	}
}
