use crate::loaded_manager::LoadedManager;
use crate::{camera, lod};

use common::IndexData;
use common::IndexNode;
use math::Vector;
pub use render::gpu;

pub struct Tree {
	pub root: Node,
	pub camera: camera::Camera,
	pub loaded_manager: LoadedManager,
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
	pub fn new(node: &IndexNode, state: &render::State, path: &String) -> Self {
		let data = match &node.data {
			IndexData::Branch(index_children) => {
				let mut children: [_; 8] = Default::default();
				for (i, child) in index_children.iter().enumerate() {
					if let Some(child) = child {
						children[i] = Some(Node::new(child, state, path));
					}
				}
				Data::Branch { children: Box::new(children) }
			},
			IndexData::Leaf() => Data::Leaf(),
		};

		Node {
			corner: node.position.into(),
			size: node.size,
			index: node.index,
			data,
		}
	}

	pub fn render<'a, 'b: 'a>(
		&'a self,
		render_pass: &mut render::RenderPass<'a>,
		view_checker: &mut lod::Checker,
		camera: &camera::Camera,
		state: &render::State,
		loaded_manager: &'b LoadedManager,
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
					loaded_manager.render(self.index, render_pass);
				} else {
					view_checker.level_down();
					for child in children.iter() {
						if let Some(child) = child {
							child.render(render_pass, view_checker, camera, state, loaded_manager);
						}
					}
					view_checker.level_up();
				}
			},
			Data::Leaf() => loaded_manager.render(self.index, render_pass),
		}
	}

	pub fn can_render_children(
		children: &Box<[Option<Node>; 8]>,
		loaded_manager: &LoadedManager,
		camera: &camera::Camera,
	) -> bool {
		let mut count = 0;
		for child in children.iter() {
			if let Some(child) = child {
				if !camera.inside_frustrum(child.corner, child.size) {
					continue;
				}
				if !loaded_manager.exist(child.index) {
					count += 1;
				}
			}
		}
		count < 3
	}

	pub fn update(
		&mut self,
		state: &render::State,
		view_checker: &mut lod::Checker,
		camera: &camera::Camera,
		loaded_manager: &mut LoadedManager,
	) {
		if !camera.inside_moved_frustrum(self.corner, self.size, -100.0) {
			self.clear(loaded_manager);
			return;
		}
		match &mut self.data {
			Data::Branch { children } => {
				if view_checker.should_render(self.corner, self.size, camera) {
					loaded_manager.request(self.index);
				} else {
					if !loaded_manager.exist(self.index) {
						loaded_manager.request(self.index);
					}
					view_checker.level_down();
					for child in children.iter_mut() {
						if let Some(child) = child {
							child.update(state, view_checker, camera, loaded_manager);
						}
					}
					view_checker.level_up();
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
				for child in children.iter() {
					if let Some(child) = child {
						child.clear(loaded_manager);
					}
				}
			},
			Data::Leaf() => {},
		}
	}
}

impl render::Renderable for Tree {
	fn render<'a, 'b: 'a>(&'a self, render_pass: &mut render::RenderPass<'a>, state: &'b render::State) {
		let mut checker = lod::Checker::new(&self.camera.lod);
		self.root.render(
			render_pass,
			&mut checker,
			&self.camera,
			state,
			&self.loaded_manager,
		);
	}
}
