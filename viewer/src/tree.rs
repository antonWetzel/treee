use std::collections::HashSet;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use crate::loaded_manager::LoadedManager;
use crate::reader::Reader;
use crate::segment::{MeshRender, Segment};

use math::{Vector, X, Y, Z};
use project::IndexNode;
use project::{IndexData, Project};
use render::{LinesRenderExt, MeshRenderExt, PointCloudExt, Window};
use window::tree::Tree;
use window::{camera, lod, tree::LookupName, State};

pub struct ProjectScene {
	pub loaded_manager: LoadedManager,
	pub root: Node,
	pub segment: Option<Segment>,
	pub segments: Reader,
}

pub struct Node {
	data: Data,
	pub corner: Vector<3, f32>,
	pub size: f32,
	index: usize,

	lines: render::Lines,
}

pub struct ProjectTree(pub Tree<ProjectScene>);
impl std::ops::Deref for ProjectTree {
	type Target = Tree<ProjectScene>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl std::ops::DerefMut for ProjectTree {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

pub enum Data {
	Branch {
		children: Box<[Option<Node>; 8]>,
		segments: HashSet<NonZeroU32>,
	},
	Leaf {
		segments: HashSet<NonZeroU32>,
	},
}

impl Node {
	pub fn new(node: &IndexNode, state: &State) -> Self {
		let data = match &node.data {
			IndexData::Branch { children: index_children } => {
				let mut children: [_; 8] = Default::default();
				let mut segments = HashSet::new();
				for (i, child) in index_children
					.iter()
					.enumerate()
					.filter_map(|(i, child)| child.as_ref().map(|c| (i, c)))
				{
					let node = Self::new(child, state);
					node.get_segments(&mut segments);
					children[i] = Some(node);
				}

				Data::Branch { children: Box::new(children), segments }
			},
			IndexData::Leaf { segments } => Data::Leaf { segments: segments.clone() },
		};

		let points = [
			node.position + Vector::new([0.0, 0.0, 0.0]),
			node.position + Vector::new([node.size, 0.0, 0.0]),
			node.position + Vector::new([node.size, 0.0, node.size]),
			node.position + Vector::new([0.0, 0.0, node.size]),
			node.position + Vector::new([0.0, node.size, 0.0]),
			node.position + Vector::new([node.size, node.size, 0.0]),
			node.position + Vector::new([node.size, node.size, node.size]),
			node.position + Vector::new([0.0, node.size, node.size]),
		];

		let indices = [
			0, 1, 1, 2, 2, 3, 3, 0, 4, 5, 5, 6, 6, 7, 7, 4, 0, 4, 1, 5, 2, 6, 3, 7,
		];

		Self {
			lines: render::Lines::new(state, &points, &indices),
			corner: node.position,
			size: node.size,
			index: node.index as usize,
			data,
		}
	}

	fn get_segments(&self, set: &mut HashSet<NonZeroU32>) {
		match &self.data {
			Data::Branch { children: _, segments } => {
				for segment in segments.iter().copied() {
					set.insert(segment);
				}
			},
			Data::Leaf { segments } => {
				for &segment in segments {
					set.insert(segment);
				}
			},
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
			Data::Branch { children, segments: _ } => {
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
			Data::Leaf { segments: _ } => {
				loaded_manager.render(self.index, point_cloud_pass);
			},
		}
	}

	pub fn render_lines<'a>(
		&'a self,
		lines_pass: &mut render::LinesPass<'a>,
		view_checker: lod::Checker,
		camera: &camera::Camera,
		loaded_manager: &'a LoadedManager,
	) {
		if !camera.inside_frustrum(self.corner, self.size) {
			return;
		}
		match &self.data {
			Data::Branch { children, segments: _ } => {
				if loaded_manager.exist(self.index)
					&& (view_checker.should_render(self.corner, self.size, camera)
						|| !Self::can_render_children(children, loaded_manager, camera))
				{
					self.lines.render(lines_pass);
				} else {
					let view_checker = view_checker.level_down();
					for child in children.iter().flatten() {
						child.render_lines(lines_pass, view_checker, camera, loaded_manager);
					}
				}
			},
			Data::Leaf { segments: _ } => {
				self.lines.render(lines_pass);
			},
		}
	}

	pub fn can_render_children(
		children: &[Option<Self>; 8],
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
		if !camera.inside_moved_frustrum(self.corner, self.size, -10.0) {
			self.clear(loaded_manager);
			return;
		}
		match &mut self.data {
			Data::Branch { children, segments: _ } => {
				if view_checker.should_render(self.corner, self.size, camera) {
					loaded_manager.request(self.index);
					for child in children.iter_mut().flatten() {
						child.clear(loaded_manager);
					}
				} else if !loaded_manager.exist(self.index) {
					loaded_manager.request(self.index);
				} else {
					let view_checker = view_checker.level_down();
					for child in children.iter_mut().flatten() {
						child.update(view_checker, camera, loaded_manager);
					}
				}
			},
			&mut Data::Leaf { segments: _ } => {
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
			Data::Branch { children, segments: _ } => {
				for child in children.iter().flatten() {
					child.clear(loaded_manager);
				}
			},
			Data::Leaf { segments: _ } => {},
		}
	}

	//https://tavianator.com/2011/ray_box.html
	pub fn raycast_distance(&self, start: Vector<3, f32>, direction: Vector<3, f32>) -> Option<f32> {
		let mut t_min = f32::NEG_INFINITY;
		let mut t_max = f32::INFINITY;

		for dir in [X, Y, Z] {
			if direction[dir] != 0.0 {
				let tx_1 = (self.corner[dir] - start[dir]) / direction[dir];
				let tx_2 = (self.corner[dir] + self.size - start[dir]) / direction[dir];

				t_min = t_min.max(tx_1.min(tx_2));
				t_max = t_max.min(tx_1.max(tx_2));
			}
		}

		(t_max >= t_min).then_some(t_min)
	}

	pub fn raycast(&self, start: Vector<3, f32>, direction: Vector<3, f32>, reader: &mut Reader) -> Option<NonZeroU32> {
		match &self.data {
			Data::Branch { children, segments: _ } => {
				let mut order = Vec::new();
				for child in children.iter().flatten() {
					let Some(dist) = child.raycast_distance(start, direction) else {
						continue;
					};
					order.push((child, dist));
				}

				order.sort_unstable_by(|(_, dist_a), (_, dist_b)| dist_a.total_cmp(dist_b));

				order
					.into_iter()
					.find_map(|(child, _)| child.raycast(start, direction, reader))
			},
			Data::Leaf { .. } => {
				let mut best = None;
				let mut best_dist = f32::MAX;
				let data = reader.get_points(self.index);
				let segments = reader.get_property(self.index);

				for (point, segment) in data.into_iter().zip(segments) {
					let diff = point.position - start;
					let diff_length = diff.length();
					if diff_length >= best_dist {
						continue;
					}
					let cos = direction.dot(diff.normalized());
					let sin = (1.0 - cos * cos).sqrt();
					let distance = sin * diff_length;
					if distance < point.size {
						let l = cos * diff_length;
						if l < best_dist {
							best = Some(NonZeroU32::new(segment).unwrap());
							best_dist = l;
						}
					}
				}
				best
			},
		}
	}
}

impl ProjectTree {
	pub fn new(
		state: Arc<State>,
		project: &Project,
		path: Option<PathBuf>,
		property: (String, String, u32),
		window: &Window,
	) -> Self {
		let scene = ProjectScene {
			root: Node::new(&project.root, &state),
			segment: None,
			segments: path
				.clone()
				.map(|path| {
					let mut segments = path.clone();
					segments.push("segments");
					Reader::new(segments, &property.0)
				})
				.unwrap_or(Reader::fake()),
			loaded_manager: LoadedManager::new(state.clone(), path, &property.0),
		};
		Self(Tree::new(state, property, window, scene))
	}

	pub fn update_lookup(&mut self, state: &State) {
		self.lookup = render::Lookup::new_png(state, self.lookup_name.data(), self.property.2);
	}

	pub fn raycast(
		&mut self,
		start: Vector<3, f32>,
		direction: Vector<3, f32>,
		reader: &mut Reader,
	) -> Option<NonZeroU32> {
		self.scene.root.raycast_distance(start, direction)?;
		self.scene.root.raycast(start, direction, reader)
	}

	pub fn update(&mut self) {
		self.0.scene.root.update(
			lod::Checker::new(&self.0.camera.lod),
			&self.0.camera,
			&mut self.0.scene.loaded_manager,
		);
	}
}

impl render::PointCloudRender for ProjectTree {
	fn render<'a>(&'a self, point_cloud_pass: &mut render::PointCloudPass<'a>) {
		self.scene.root.render(
			point_cloud_pass,
			lod::Checker::new(&self.camera.lod),
			&self.camera,
			&self.scene.loaded_manager,
		);
	}
}

impl render::LinesRender for ProjectTree {
	fn render<'a>(&'a self, lines_pass: &mut render::LinesPass<'a>) {
		self.scene.root.render_lines(
			lines_pass,
			lod::Checker::new(&self.camera.lod),
			&self.camera,
			&self.scene.loaded_manager,
		);
	}
}

impl render::RenderEntry<State> for ProjectTree {
	fn background(&self) -> Vector<3, f32> {
		self.background
	}

	fn render<'a>(&'a mut self, state: &'a State, render_pass: &mut render::RenderPass<'a>) {
		if let Some(segment) = &self.scene.segment {
			match segment.render {
				MeshRender::Points => render_pass.render_point_clouds(
					segment,
					state,
					&self.camera.gpu,
					&self.lookup,
					&self.environment,
				),
				MeshRender::Mesh => render_pass.render_meshes(segment, state, &self.camera.gpu, &self.lookup),
				MeshRender::MeshLines => {
					render_pass.render_meshes(segment, &state.mesh_line, &self.camera.gpu, &self.lookup)
				},
			}
		} else {
			render_pass.render_point_clouds(
				self,
				state,
				&self.camera.gpu,
				&self.lookup,
				&self.environment,
			);
			if self.voxels_active {
				render_pass.render_lines(self, state, &self.camera.gpu);
			}
		}
	}

	fn post_process<'a>(&'a mut self, _state: &'a State, render_pass: &mut render::RenderPass<'a>) {
		if self.eye_dome_active {
			render_pass.render(&self.eye_dome, ());
		}
	}
}
