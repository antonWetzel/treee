use std::collections::HashSet;
use std::io::Read;
use std::num::NonZeroU32;
use std::ops::Not;
use std::path::{ Path, PathBuf };

use crate::loaded_manager::LoadedManager;
use crate::segment::Segment;
use crate::state::State;
use crate::{ camera, lod };

use common::IndexNode;
use common::{ IndexData, Project };
use math::{ Dimension, Vector, X, Y, Z };
use render::{ Window, LinesRenderExt, MeshRenderExt, PointCloudExt };


pub const DEFAULT_BACKGROUND: Vector<3, f32> = Vector::new([0.1, 0.2, 0.3]);


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LookupName {
	Warm,
	Cold,
	#[allow(dead_code)]
	Wood,
}


impl LookupName {
	pub fn data(self) -> &'static [u8] {
		match self {
			LookupName::Warm => include_bytes!("../assets/grad_warm.png"),
			LookupName::Cold => include_bytes!("../assets/grad_cold.png"),
			LookupName::Wood => include_bytes!("../assets/grad_wood.png"),
		}
	}
}


pub struct Tree {
	pub root: Node,
	pub camera: camera::Camera,
	pub loaded_manager: LoadedManager,
	pub lookup: render::Lookup,
	pub environment: render::PointCloudEnvironment,
	pub background: Vector<3, f32>,
	pub segment: Option<Segment>,

	pub lookup_name: LookupName,
	pub eye_dome: render::EyeDome,
	pub eye_dome_active: bool,
	pub voxels_active: bool,

	pub property: (String, String),
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
					.filter_map(|(i, child)| child.as_ref().map(|c| (i, c))) {
					let node = Node::new(child, state);
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

		let indices = [0, 1, 1, 2, 2, 3, 3, 0, 4, 5, 5, 6, 6, 7, 7, 4, 0, 4, 1, 5, 2, 6, 3, 7];

		Node {
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
				} else {
					if !loaded_manager.exist(self.index) {
						loaded_manager.request(self.index);
					} else {
						let view_checker = view_checker.level_down();
						for child in children.iter_mut().flatten() {
							child.update(view_checker, camera, loaded_manager);
						}
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
			Data::Leaf { segments: _ } => { },
		}
	}


	pub fn raycast_distance(&self, start: Vector<3, f32>, direction: Vector<3, f32>) -> Option<f32> {
		let size = Vector::new([self.size, self.size, self.size]);
		let walls = [
			(self.corner, size, X),
			(self.corner, size, Y),
			(self.corner, size, Z),
			(self.corner + size, -size, X),
			(self.corner + size, -size, Y),
			(self.corner + size, -size, Z),
		];
		walls
			.into_iter()
			.map(|(corner, wall, dimension)| raycast_check(start, direction, corner, wall, dimension))
			.fold(None, |acc, v| match (acc, v) {
				(Some(acc), Some(v)) => Some(acc.min(v)),
				(Some(v), None) | (None, Some(v)) => Some(v),
				(None, None) => None,
			})
	}


	pub fn raycast(
		&self,
		start: Vector<3, f32>,
		direction: Vector<3, f32>,
		path: &Path,
		checked: &mut HashSet<NonZeroU32>,
	) -> Option<(NonZeroU32, f32)> {
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

				let mut best_distance = f32::MAX;
				let mut best = None;
				for (child, dist) in order {
					if dist >= best_distance {
						break;
					}
					let Some((segment, dist)) = child.raycast(start, direction, path, checked) else {
						continue;
					};
					if dist < best_distance {
						best_distance = dist;
						best = Some(segment);
					}
				}
				best.map(|best| (best, best_distance))
			},
			Data::Leaf { segments } => {
				let mut best = None;
				let mut best_dist = f32::MAX;
				for &segment in segments {
					if checked.contains(&segment) {
						continue;
					}

					let mut path = path.to_path_buf();
					path.push(format!("{}", segment));
					path.push("points.data");
					let Ok(mut file) = std::fs::OpenOptions::new().read(true).open(&path) else {
						continue;
					};
					let length = file.metadata().unwrap().len();
					let mut data
						= bytemuck::zeroed_vec::<render::Point>(length as usize / std::mem::size_of::<render::Point>());
					file.read_exact(bytemuck::cast_slice_mut(&mut data))
						.unwrap();

					for point in data {
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
								best = Some(segment);
								best_dist = l;
							}
						}
					}

					checked.insert(segment);
				}
				best.map(|best| (best, best_dist))
			},
		}
	}
}


impl Tree {
	pub fn new(state: &'static State, project: &Project, path: PathBuf, property: (String, String), window: &Window) -> Self {
		let lookup_name = LookupName::Warm;
		Self {
			background: DEFAULT_BACKGROUND,
			camera: camera::Camera::new(state, window.get_aspect()),
			root: Node::new(&project.root, state),
			loaded_manager: LoadedManager::new(state, path, &property.0),
			lookup_name,
			lookup: render::Lookup::new_png(state, lookup_name.data()),
			environment: render::PointCloudEnvironment::new(state, u32::MIN, u32::MAX, 1.0),
			segment: None,
			eye_dome: render::EyeDome::new(state, window.config(), window.depth_texture(), 0.7),
			eye_dome_active: true,
			voxels_active: false,

			property,
		}
	}


	pub fn update_lookup(&mut self, state: &'static State) {
		self.lookup = render::Lookup::new_png(state, self.lookup_name.data());
	}


	pub fn raycast(&self, start: Vector<3, f32>, direction: Vector<3, f32>, path: &Path) -> Option<NonZeroU32> {
		self.root.raycast_distance(start, direction)?;
		let mut checked = HashSet::new();
		self.root
			.raycast(start, direction, path, &mut checked)
			.map(|(seg, _)| seg)
	}


	pub fn update(&mut self) {
		self.root.update(
			lod::Checker::new(&self.camera.lod),
			&self.camera,
			&mut self.loaded_manager,
		);
	}
}


fn raycast_check(
	start: Vector<3, f32>,
	direction: Vector<3, f32>,
	corner: Vector<3, f32>,
	wall: Vector<3, f32>, // wall[dimension] is ignored and assumed to be zero
	dimension: Dimension,
) -> Option<f32> {
	if direction[dimension].abs() < f32::EPSILON {
		return None;
	}

	let diff = corner - start;

	let dist = diff[dimension] / direction[dimension];
	if dist < 0.0 {
		return None;
	}
	let intersect = direction * dist - diff;

	if (0.0..1.0)
		.contains(&(intersect[dimension.previous(Z)] / wall[dimension.previous(Z)]))
		.not()
	{
		return None;
	}

	if (0.0..1.0)
		.contains(&(intersect[dimension.next(Z)] / wall[dimension.next(Z)]))
		.not()
	{
		return None;
	}

	Some(dist)
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


impl render::RenderEntry<State> for Tree {
	fn background(&self) -> Vector<3, f32> {
		self.background
	}


	fn render<'a>(&'a mut self, state: &'a State, render_pass: &mut render::RenderPass<'a>) {
		if let Some(segment) = &self.segment {
			if segment.render_mesh {
				render_pass.render_meshes(segment, state, &self.camera.gpu, &self.lookup);
			} else {
				render_pass.render_point_clouds(segment, state, &self.camera.gpu, &self.lookup, &self.environment);
			}
		} else {
			render_pass.render_point_clouds(self, state, &self.camera.gpu, &self.lookup, &self.environment);
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
