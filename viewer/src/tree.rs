use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use crate::loaded_manager::LoadedManager;
use crate::reader::Reader;
use crate::segment::{MeshRender, Segment};
use crate::state::State;
use crate::{camera, lod};

use nalgebra as na;
use project::{IndexData, Project};
use project::{IndexNode, Property};
use render::{LinesRender, LinesRenderExt, MeshRenderExt, PointCloudExt, Window};

pub const DEFAULT_BACKGROUND: na::Point3<f32> = na::Point3::new(0.1, 0.2, 0.3);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LookupName {
	Warm,
	Cold,
	Turbo,
}

impl LookupName {
	pub fn data(self) -> &'static [u8] {
		match self {
			Self::Warm => include_bytes!("../assets/grad_warm.png"),
			Self::Cold => include_bytes!("../assets/grad_cold.png"),
			Self::Turbo => include_bytes!("../assets/grad_turbo.png"),
		}
	}
}

pub struct Tree {
	pub root: Node,
	pub camera: camera::Camera,
	pub loaded_manager: LoadedManager,
	pub lookup: render::Lookup,
	pub environment: render::PointCloudEnvironment,
	pub background: na::Point3<f32>,
	pub segment: Option<Segment>,

	pub lookup_name: LookupName,
	pub eye_dome: render::EyeDome,
	pub eye_dome_active: bool,
	pub voxels_active: bool,

	pub property: Property,

	pub segments: Reader,
}

#[derive(Debug)]
pub struct Node {
	data: Data,
	pub corner: na::Point3<f32>,
	pub size: f32,
	index: usize,

	lines: render::Lines,
}

#[derive(Debug)]
pub enum Data {
	Branch(Box<[Option<Node>; 8]>),
	Leaf,
}

impl Node {
	pub fn new(node: &IndexNode, state: &State) -> Self {
		let data = match &node.children {
			IndexData::Branch(index_children) => {
				let mut children: [_; 8] = Default::default();
				for (i, child) in index_children
					.iter()
					.enumerate()
					.filter_map(|(i, child)| child.as_ref().map(|c| (i, c)))
				{
					let node = Self::new(child, state);
					children[i] = Some(node);
				}

				Data::Branch(Box::new(children))
			},
			IndexData::Leaf => Data::Leaf,
		};

		let points = [
			node.position + na::vector![0.0, 0.0, 0.0],
			node.position + na::vector![node.size, 0.0, 0.0],
			node.position + na::vector![node.size, 0.0, node.size],
			node.position + na::vector![0.0, 0.0, node.size],
			node.position + na::vector![0.0, node.size, 0.0],
			node.position + na::vector![node.size, node.size, 0.0],
			node.position + na::vector![node.size, node.size, node.size],
			node.position + na::vector![0.0, node.size, node.size],
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
			Data::Branch(children) => {
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
			Data::Leaf => {
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
			Data::Branch(children) => {
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
			Data::Leaf => {
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
			Data::Branch(children) => {
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
			&mut Data::Leaf => {
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
			Data::Branch(children) => {
				for child in children.iter().flatten() {
					child.clear(loaded_manager);
				}
			},
			Data::Leaf => {},
		}
	}

	//https://tavianator.com/2011/ray_box.html
	pub fn raycast_distance(&self, start: na::Point3<f32>, direction: na::Vector3<f32>) -> Option<f32> {
		let mut t_min = f32::NEG_INFINITY;
		let mut t_max = f32::INFINITY;

		for dir in 0..3 {
			if direction[dir] != 0.0 {
				let tx_1 = (self.corner[dir] - start[dir]) / direction[dir];
				let tx_2 = (self.corner[dir] + self.size - start[dir]) / direction[dir];

				t_min = t_min.max(tx_1.min(tx_2));
				t_max = t_max.min(tx_1.max(tx_2));
			}
		}

		(t_max >= t_min && t_max >= 0.0).then_some(t_max)
	}

	pub fn raycast(
		&self,
		start: na::Point3<f32>,
		direction: na::Vector3<f32>,
		reader: &mut Reader,
	) -> Option<NonZeroU32> {
		match &self.data {
			Data::Branch(children) => {
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
			Data::Leaf => {
				let mut best = None;
				let mut best_dist = f32::MAX;
				let data = reader.get_points(self.index);
				let segments = reader.get_property(self.index);

				for (point, segment) in data.into_iter().zip(segments) {
					let diff = point.position - start;
					let diff_length = diff.norm();
					if diff_length >= best_dist {
						continue;
					}
					let cos = direction.dot(&diff.normalize());
					let sin = (1.0 - cos * cos).sqrt();
					let distance = sin * diff_length;
					if distance > point.size {
						continue;
					}
					let l = cos * diff_length;
					if l < 0.0 || best_dist < l {
						continue;
					}
					best = Some(NonZeroU32::new(segment).unwrap());
					best_dist = l;
				}
				best
			},
		}
	}
}

impl Tree {
	pub fn new(
		state: Arc<State>,
		project: &Project,
		path: Option<PathBuf>,
		property: Property,
		window: &Window,
	) -> Self {
		let lookup_name = LookupName::Warm;

		Self {
			background: DEFAULT_BACKGROUND,
			camera: camera::Camera::new(&state, window.get_aspect()),
			root: Node::new(&project.root, &state),
			lookup_name,
			lookup: render::Lookup::new_png(&state, lookup_name.data(), property.max),
			environment: render::PointCloudEnvironment::new(&state, u32::MIN, u32::MAX, 1.0),
			segment: None,
			eye_dome: render::EyeDome::new(&state, window.config(), window.depth_texture(), 0.7),
			eye_dome_active: true,
			voxels_active: false,
			segments: path
				.clone()
				.map(|path| {
					let mut segments = path.clone();
					segments.push("segments");
					Reader::new(segments, &property.storage_name)
				})
				.unwrap_or(Reader::fake()),

			loaded_manager: LoadedManager::new(state, path, &property.storage_name),
			property,
		}
	}

	pub fn update_lookup(&mut self, state: &State) {
		self.lookup = render::Lookup::new_png(state, self.lookup_name.data(), self.property.max);
	}

	pub fn raycast(
		&mut self,
		start: na::Point3<f32>,
		direction: na::Vector3<f32>,
		reader: &mut Reader,
	) -> Option<NonZeroU32> {
		self.root.raycast_distance(start, direction)?;
		self.root.raycast(start, direction, reader)
	}

	pub fn update(&mut self) {
		self.root.update(
			lod::Checker::new(&self.camera.lod),
			&self.camera,
			&mut self.loaded_manager,
		);
	}

	pub fn render<'a>(&'a self, state: &'a State, render_pass: &mut render::RenderPass<'a>) {
		if let Some(segment) = &self.segment {
			if segment.show_grid {
				render_pass.render_lines(&segment.grid, &state.lines, &self.camera.gpu);
			}
			match segment.render {
				MeshRender::Points => render_pass.render_point_clouds(
					segment,
					&state.pointcloud,
					&self.camera.gpu,
					&self.lookup,
					&self.environment,
				),
				MeshRender::Mesh => render_pass.render_meshes(segment, &state.mesh, &self.camera.gpu, &self.lookup),
				MeshRender::MeshLines => {
					render_pass.render_meshes(segment, &state.mesh_line, &self.camera.gpu, &self.lookup)
				},
			}
		} else {
			render_pass.render_point_clouds(
				self,
				&state.pointcloud,
				&self.camera.gpu,
				&self.lookup,
				&self.environment,
			);
			if self.voxels_active {
				render_pass.render_lines(self, &state.lines, &self.camera.gpu);
			}
		}
	}

	pub fn post_process<'a>(&'a self, _state: &'a State, render_pass: &mut render::RenderPass<'a>) {
		if self.eye_dome_active {
			render_pass.render(&self.eye_dome, ());
		}
	}
}

impl render::PointCloudRender for Tree {
	fn render<'a>(&'a self, point_cloud_pass: &mut render::PointCloudPass<'a>) {
		self.root.render(
			point_cloud_pass,
			lod::Checker::new(&self.camera.lod),
			&self.camera,
			&self.loaded_manager,
		);
	}
}

impl render::LinesRender for Tree {
	fn render<'a>(&'a self, lines_pass: &mut render::LinesPass<'a>) {
		self.root.render_lines(
			lines_pass,
			lod::Checker::new(&self.camera.lod),
			&self.camera,
			&self.loaded_manager,
		);
	}
}
