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
use window::tree::{Scene, Tree, TreeContext};
use window::{camera, lod, State};

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

// trait TreeExt {
// 	pub fn new_project()
// }

impl ProjectScene {
	pub fn raycast(
		&mut self,
		start: Vector<3, f32>,
		direction: Vector<3, f32>,
		reader: &mut Reader,
	) -> Option<NonZeroU32> {
		self.root.raycast_distance(start, direction)?;
		self.root.raycast(start, direction, reader)
	}
}

impl render::PointCloudRender<TreeContext> for ProjectScene {
	fn render<'a>(&'a self, context: &'a TreeContext, point_cloud_pass: &mut render::PointCloudPass<'a>) {
		self.root.render(
			point_cloud_pass,
			lod::Checker::new(&context.camera.lod),
			&context.camera,
			&self.loaded_manager,
		);
	}
}

impl Scene for ProjectScene {
	fn render<'a>(&'a self, state: &'a State, tree: &'a Tree<Self>, render_pass: &mut render::RenderPass<'a>) {
		if let Some(segment) = &self.segment {
			match segment.render {
				MeshRender::Points => render_pass.render_point_clouds(
					segment,
					state,
					&(),
					&tree.context.camera.gpu,
					&tree.context.lookup,
					&tree.context.environment,
				),
				MeshRender::Mesh => render_pass.render_meshes(
					segment,
					state,
					&tree.context.camera.gpu,
					&tree.context.lookup,
				),
				MeshRender::MeshLines => render_pass.render_meshes(
					segment,
					&state.mesh_line,
					&tree.context.camera.gpu,
					&tree.context.lookup,
				),
			}
		} else {
			render_pass.render_point_clouds(
				self,
				state,
				&tree.context,
				&tree.context.camera.gpu,
				&tree.context.lookup,
				&tree.context.environment,
			);
			if tree.context.voxels_active {
				render_pass.render_lines(self, state, &tree.context, &tree.context.camera.gpu);
			}
		}
	}
}

impl render::LinesRender<TreeContext> for ProjectScene {
	fn render<'a>(&'a self, context: &'a TreeContext, lines_pass: &mut render::LinesPass<'a>) {
		self.root.render_lines(
			lines_pass,
			lod::Checker::new(&context.camera.lod),
			&context.camera,
			&self.loaded_manager,
		);
	}
}
