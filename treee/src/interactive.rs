use nalgebra as na;
use std::{
	collections::{HashMap, HashSet},
	hash::Hash,
	ops::Not,
};

use crate::{
	calculations::{DisplayModus, SegmentData},
	environment,
	program::Event,
};

pub const DELETED_INDEX: u32 = 0;

pub struct Interactive {
	pub segments: HashMap<u32, SegmentData>,
	pub deleted: SegmentData,
	pub display: DisplayModus,
	sender: crossbeam::channel::Sender<Event>,

	pub modus: Modus,
	pub show_deleted: bool,
	draw_radius: f32,
	pub white_lookup: render::Lookup,

	#[cfg(not(target_arch = "wasm32"))]
	pub source_location: String,
	world_offset: na::Point3<f64>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InteractiveSave {
	pub segments: HashMap<u32, SegmentData>,
	pub deleted: SegmentData,
	pub world_offset: na::Point3<f64>,
}

#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_LOCATION: &str = "+proj=utm\n+ellps=GRS80\n+zone=32";

impl SegmentData {
	//https://tavianator.com/2011/ray_box.html
	pub fn raycast_distance(&self, start: na::Point3<f32>, direction: na::Vector3<f32>) -> Option<(f32, f32)> {
		let mut t_min = f32::NEG_INFINITY;
		let mut t_max = f32::INFINITY;

		for dir in 0..3 {
			if direction[dir] != 0.0 {
				let tx_1 = (self.min[dir] - start[dir]) / direction[dir];
				let tx_2 = (self.max[dir] - start[dir]) / direction[dir];

				t_min = t_min.max(tx_1.min(tx_2));
				t_max = t_max.min(tx_1.max(tx_2));
			}
		}

		(t_max >= t_min && t_max >= 0.0).then_some((t_min, t_max))
	}

	pub fn exact_distance(&self, start: na::Point3<f32>, direction: na::Vector3<f32>) -> Option<f32> {
		let mut found = false;
		let mut best_dist = f32::MAX;

		for &point in self.points.iter() {
			let diff = point - start;
			let diff_length = diff.norm();
			if diff_length >= best_dist {
				continue;
			}
			let cos = direction.dot(&diff.normalize());
			let sin = (1.0 - cos * cos).sqrt();
			let distance = sin * diff_length;
			// todo: replace 0.1 with real point size
			if distance > 0.1 {
				continue;
			}
			let l = cos * diff_length;
			if l < 0.0 || best_dist < l {
				continue;
			}
			found = true;
			best_dist = l;
		}
		found.then_some(best_dist)
	}

	pub fn remove(&mut self, center: na::Point3<f32>, radius: f32, target: &mut Vec<na::Point3<f32>>) -> bool {
		for dim in 0..3 {
			if ((self.min[dim] - radius)..(self.max[dim] + radius))
				.contains(&center[dim])
				.not()
			{
				return false;
			}
		}
		let r2 = radius * radius;
		let mut changed = false;
		self.points.retain(|&p| {
			if (p - center).norm_squared() > r2 {
				return true;
			}
			target.push(p);
			changed = true;
			false
		});

		if changed {
			self.update_min_max();
		}
		changed
	}

	fn update_min_max(&mut self) {
		if self.points.is_empty() {
			return;
		}
		(self.min, self.max) = (self.points[0], self.points[0]);
		for &p in self.points.iter() {
			for dim in 0..3 {
				self.min[dim] = self.min[dim].min(p[dim]);
				self.max[dim] = self.max[dim].max(p[dim]);
			}
		}
	}

	#[cfg(not(target_arch = "wasm32"))]
	fn update_location(&mut self, world_offset: na::Point3<f64>, proj: &proj4rs::Proj) {
		let to = proj4rs::Proj::from_proj_string("+proj=latlong +ellps=GRS80").unwrap();
		let mut point = (
			world_offset.x + ((self.min.x + self.max.x) / 2.0) as f64,
			-(world_offset.z + ((self.min.z + self.max.z) / 2.0) as f64),
		);
		proj4rs::transform::transform(proj, &to, &mut point).unwrap();
		self.coords = Some(point);
	}

	pub fn height(&self) -> f32 {
		self.max.y - self.min.y
	}
}

impl Interactive {
	pub fn new(
		segments: HashMap<u32, SegmentData>,
		state: &render::State,
		display: DisplayModus,
		world_offset: na::Point3<f64>,
	) -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();
		let deleted = SegmentData::new(Vec::new());
		let white_lookup = render::Lookup::new_png(state, include_bytes!("../assets/white.png"), u32::MAX);

		let interactive = Self {
			segments,
			modus: Modus::SelectView,
			deleted,
			draw_radius: 0.5,
			display,
			sender,
			show_deleted: false,
			#[cfg(not(target_arch = "wasm32"))]
			source_location: DEFAULT_LOCATION.into(),
			world_offset,
			white_lookup,
		};

		(interactive, receiver)
	}

	pub fn load(source: environment::Source, state: &render::State) -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();
		sender
			.send(Event::Lookup {
				bytes: include_bytes!("../assets/grad_turbo.png"),
				max: u32::MAX,
			})
			.unwrap();

		let reader = environment::reader(&source);
		let save = bincode::deserialize_from::<_, InteractiveSave>(reader).unwrap();
		let white_lookup = render::Lookup::new_png(&state, include_bytes!("../assets/white.png"), u32::MAX);

		let mut segments = HashMap::new();
		for (idx, data) in save.segments {
			data.update_render(idx, &sender);
			segments.insert(idx, data);
		}
		save.deleted.update_render(DELETED_INDEX, &sender);

		let interactive = Self {
			segments,
			modus: Modus::SelectView,
			sender,
			deleted: save.deleted,
			draw_radius: 0.5,
			display: DisplayModus::Segment,
			show_deleted: false,
			#[cfg(not(target_arch = "wasm32"))]
			source_location: DEFAULT_LOCATION.into(),
			world_offset: save.world_offset,
			white_lookup,
		};

		(interactive, receiver)
	}

	pub fn ui(&mut self, ui: &mut egui::Ui, state: &render::State) {
		ui.separator();
		if ui
			.add_sized([ui.available_width(), 0.0], egui::Button::new("Save"))
			.clicked()
		{
			let mut segments = HashMap::new();
			for (&idx, segment) in self.segments.iter() {
				segments.insert(idx, segment.clone());
			}
			let save = InteractiveSave {
				segments,
				deleted: self.deleted.clone(),
				world_offset: self.world_offset,
			};
			environment::save(save);
		}

		self.display.ui(ui);

		if let Modus::View(..) = self.modus {
			// skip
		} else {
			ui.separator();
			ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Modus"));
			if ui
				.add(egui::RadioButton::new(
					matches!(self.modus, Modus::SelectView),
					"View",
				))
				.clicked()
			{
				self.modus = Modus::SelectView;
			}
			if ui
				.add(egui::RadioButton::new(
					matches!(self.modus, Modus::SelectDraw | Modus::Draw(_)),
					"Change",
				))
				.clicked()
			{
				self.modus = Modus::SelectDraw;
			}
			if ui
				.add(egui::RadioButton::new(
					matches!(self.modus, Modus::SelectCombine | Modus::Combine(_)),
					"Combine",
				))
				.clicked()
			{
				self.modus = Modus::SelectCombine;
			}
			if ui
				.add(egui::RadioButton::new(
					matches!(self.modus, Modus::Spawn),
					"Spawn",
				))
				.clicked()
			{
				self.modus = Modus::Spawn;
			}
			if ui
				.add(egui::RadioButton::new(
					matches!(self.modus, Modus::Delete),
					"Delete",
				))
				.clicked()
			{
				self.modus = Modus::Delete;
			}
		}

		ui.separator();
		ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Settings"));
		egui::Grid::new("remove grid")
			.num_columns(2)
			.show(ui, |ui| {
				ui.label("Radius");
				ui.add(
					egui::Slider::new(&mut self.draw_radius, 0.1..=10.0)
						.logarithmic(true)
						.fixed_decimals(2)
						.suffix("m"),
				);
				ui.end_row();
			});
		ui.checkbox(&mut self.show_deleted, "Show Deleted");

		#[cfg(not(target_arch = "wasm32"))]
		{
			ui.separator();
			ui.add_sized(
				[ui.available_width(), 0.0],
				egui::Label::new("Source Region"),
			);
			if ui
				.text_edit_multiline(&mut self.source_location)
				.lost_focus()
			{
				if let Modus::View(idx, _) = self.modus {
					let seg = self.segments.get_mut(&idx).unwrap();
					match proj4rs::Proj::from_proj_string(&self.source_location) {
						Ok(proj) => seg.update_location(self.world_offset, &proj),
						Err(err) => eprintln!("{}", err),
					}
				}
			};
		}

		if let Modus::View(..) = self.modus {
			ui.separator();
			ui.add_sized(
				[ui.available_width(), 0.0],
				egui::Label::new(egui::RichText::new("View").heading()),
			);
			if ui
				.add_sized([ui.available_width(), 0.0], egui::Button::new("Return"))
				.clicked()
			{
				self.modus = Modus::SelectView;
			}
		}
		match self.modus {
			Modus::SelectView => {},
			Modus::Spawn => {},
			Modus::SelectDraw | Modus::Draw(_) => {},
			Modus::SelectCombine | Modus::Combine(_) => {},
			Modus::Delete => {},
			Modus::View(idx, ref mut mesh) => {
				let segment = self.segments.get_mut(&idx).unwrap();

				ui.separator();
				ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Seperation"));
				egui::Grid::new("sep grid").num_columns(2).show(ui, |ui| {
					let mut changed = false;
					let mut stopped = false;
					let mut rel_ground = segment.info.ground_sep - segment.min.y;
					ui.label("Ground");
					let res = ui.add(
						egui::Slider::new(&mut rel_ground, 0.0..=segment.height())
							.suffix("m")
							.fixed_decimals(2),
					);
					if res.changed() {
						changed = true;
						segment.info.ground_sep = segment.min.y + rel_ground;
						segment.info.crown_sep = segment.info.crown_sep.max(segment.info.ground_sep);
					}
					if res.drag_stopped() {
						stopped = true;
					}
					ui.end_row();

					let mut rel_crown = segment.info.crown_sep - segment.min.y;
					ui.label("Crown");
					let res = ui.add(
						egui::Slider::new(&mut rel_crown, 0.0..=segment.height())
							.suffix("m")
							.fixed_decimals(2),
					);
					if res.changed() {
						changed = true;
						segment.info.crown_sep = segment.min.y + rel_crown;
						segment.info.ground_sep = segment.info.ground_sep.min(segment.info.crown_sep);
					}
					if res.drag_stopped() {
						stopped = true;
					}
					ui.end_row();

					if changed {
						segment
							.info
							.redo_diameters(&segment.points, segment.min.y, segment.max.y);
						segment.update_render(idx, &self.sender);
					}
					if let (true, Some(mesh)) = (stopped, mesh.as_mut()) {
						*mesh = convex_hull(&segment.points, segment.info.crown_sep, state);
					}
				});

				ui.separator();
				ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Diameter"));
				egui::Grid::new("diameter grid")
					.num_columns(2)
					.show(ui, |ui| {
						ui.label("Trunk");
						ui.label(format!("{}m", segment.info.trunk_diameter));
						ui.end_row();

						ui.label("Crown");
						ui.label(format!("{}m", segment.info.crown_diameter));
						ui.end_row();
					});

				if let Some((long, lat)) = segment.coords {
					ui.separator();
					ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Coordinates"));
					egui::Grid::new("coords grid")
						.num_columns(2)
						.show(ui, |ui| {
							ui.label("Lat");
							ui.label(format_degrees(lat));
							ui.end_row();

							ui.label("Long");
							ui.label(format_degrees(long));
							ui.end_row();
						});
				}

				ui.separator();
				let mut render_mesh = mesh.is_some();
				if ui.checkbox(&mut render_mesh, "Convex Hull").changed() {
					*mesh = if render_mesh {
						Some(convex_hull(&segment.points, segment.info.crown_sep, state))
					} else {
						None
					};
				}

				ui.separator();
				if ui
					.add_sized(
						[ui.available_width(), 0.0],
						egui::Button::new("Export (Todo)"),
					)
					.clicked()
				{
					println!("todo");
				}
			},
		};
	}

	fn select(&self, start: na::Point3<f32>, direction: na::Vector3<f32>) -> Option<(u32, f32)> {
		let mut potential = Vec::new();
		for (&idx, segment) in self.segments.iter() {
			let Some(distance) = segment.raycast_distance(start, direction) else {
				continue;
			};
			potential.push((idx, distance));
		}
		potential.sort_by(|a, b| a.1 .0.total_cmp(&b.1 .0));
		let mut best = None;
		let mut distance = f32::MAX;
		for (idx, (min, _)) in potential {
			if min > distance {
				break;
			}
			let Some(d) = self.segments[&idx].exact_distance(start, direction) else {
				continue;
			};
			if d < distance {
				distance = d;
				best = Some(idx);
			}
		}
		best.map(|idx| (idx, distance))
	}

	pub fn click(&mut self, start: na::Point3<f32>, direction: na::Vector3<f32>) {
		match &mut self.modus {
			Modus::SelectDraw | Modus::Draw(_) => {
				self.modus = if let Some((idx, _)) = self.select(start, direction) {
					Modus::Draw(idx)
				} else {
					Modus::SelectDraw
				};
			},
			Modus::SelectCombine | Modus::Combine(_) => {
				self.modus = if let Some((idx, _)) = self.select(start, direction) {
					Modus::Combine(idx)
				} else {
					Modus::SelectCombine
				};
			},
			Modus::Spawn => {
				let Some((_, distance)) = self.select(start, direction) else {
					return;
				};
				let hit = start + direction * distance;
				let mut points = Vec::new();
				let mut empty = Vec::new();
				for (&other, segment) in self.segments.iter_mut() {
					let seg_changed = segment.remove(hit, self.draw_radius, &mut points);
					if segment.points.is_empty() {
						empty.push(other);
					} else if seg_changed {
						segment.update_render(other, &self.sender);
					}
				}
				for empty in empty {
					self.sender.send(Event::RemovePointCloud(empty)).unwrap();
					self.segments.remove(&empty);
				}
				if points.is_empty() {
					return;
				}

				let mut idx = rand::random();
				while idx == DELETED_INDEX || self.segments.contains_key(&idx) {
					idx = rand::random();
				}
				let segment = SegmentData::new(points);
				// todo: crate render data
				self.segments.insert(idx, segment);
				self.modus = Modus::Draw(idx);
			},
			Modus::Delete => {},

			Modus::SelectView => {
				let Some((idx, _)) = self.select(start, direction) else {
					return;
				};
				let seg = self.segments.get_mut(&idx).unwrap();
				seg.info.redo_diameters(&seg.points, seg.min.y, seg.max.y);

				#[cfg(not(target_arch = "wasm32"))]
				match proj4rs::Proj::from_proj_string(&self.source_location) {
					Ok(proj) => seg.update_location(self.world_offset, &proj),
					Err(err) => eprintln!("{}", err),
				}

				self.modus = Modus::View(idx, None);
			},

			Modus::View(..) => {},
		}
	}

	pub fn drag(&mut self, start: na::Point3<f32>, direction: na::Vector3<f32>, state: &render::State) {
		match self.modus {
			Modus::Delete => {
				let Some((_, distance)) = self.select(start, direction) else {
					return;
				};
				let hit = start + direction * distance;
				let mut changed = false;
				let mut empty = Vec::new();
				for (&other, segment) in self.segments.iter_mut() {
					if segment.remove(hit, self.draw_radius, &mut self.deleted.points) {
						segment.update_render(other, &self.sender);
						segment.update_min_max();
						changed = true;
					}
					if segment.points.is_empty() {
						empty.push(other);
					}
				}
				for empty in empty {
					self.sender.send(Event::RemovePointCloud(empty)).unwrap();
					self.segments.remove(&empty);
				}
				if changed {
					self.deleted.update_min_max();
					self.deleted.update_render(DELETED_INDEX, &self.sender);
				}
			},
			Modus::Draw(idx) => {
				let Some(distance) = self
					.select(start, direction)
					.map(|(_, distance)| distance)
					.or_else(|| {
						if self.show_deleted.not() {
							return None;
						}
						self.deleted.raycast_distance(start, direction)?;
						self.deleted.exact_distance(start, direction)
					})
				else {
					return;
				};
				let hit = start + direction * distance;
				let mut target = self.segments.remove(&idx).unwrap();
				let mut changed = false;
				let mut empty = Vec::new();
				for (&other, segment) in self.segments.iter_mut() {
					if segment.remove(hit, self.draw_radius, &mut target.points) {
						segment.update_render(other, &self.sender);
						segment.update_min_max();
						changed = true;
					}
					if segment.points.is_empty() {
						empty.push(other);
					}
				}
				if self.show_deleted {
					if self
						.deleted
						.remove(hit, self.draw_radius, &mut target.points)
					{
						self.deleted.update_render(DELETED_INDEX, &self.sender);
						self.deleted.update_min_max();
						changed = true;
					}
				}

				if changed {
					target.update_render(idx, &self.sender);
					target.update_min_max();
				}
				self.segments.insert(idx, target);
				for empty in empty {
					self.sender.send(Event::RemovePointCloud(empty)).unwrap();
					self.segments.remove(&empty);
				}
			},
			Modus::View(idx, ref mut mesh) => {
				let seg = self.segments.get_mut(&idx).unwrap();
				if self.show_deleted {
					let Some(distance) = self
						.deleted
						.exact_distance(start, direction)
						.or_else(|| seg.exact_distance(start, direction))
					else {
						return;
					};
					let hit = start + direction * distance;

					if self.deleted.remove(hit, self.draw_radius, &mut seg.points) {
						seg.update_render(idx, &self.sender);
						seg.update_min_max();
						if let Some(mesh) = mesh {
							*mesh = convex_hull(&seg.points, seg.info.crown_sep, state);
						}
						self.deleted.update_render(DELETED_INDEX, &self.sender);
						self.deleted.update_min_max();
					}
				} else {
					let Some(distance) = seg.exact_distance(start, direction) else {
						return;
					};

					let hit = start + direction * distance;

					if seg.remove(hit, self.draw_radius, &mut self.deleted.points) {
						seg.update_render(idx, &self.sender);
						seg.update_min_max();
						if let Some(mesh) = mesh {
							*mesh = convex_hull(&seg.points, seg.info.crown_sep, state);
						}

						self.deleted.update_render(DELETED_INDEX, &self.sender);
						self.deleted.update_min_max();
					}
				}
			},
			Modus::Combine(idx) => {
				let Some((other, _)) = self.select(start, direction) else {
					return;
				};
				if other == idx {
					return;
				}
				self.sender.send(Event::RemovePointCloud(other)).unwrap();
				let mut other = self.segments.remove(&other).unwrap();
				let target = self.segments.get_mut(&idx).unwrap();
				target.points.append(&mut other.points);
				target.update_render(idx, &self.sender);
				target.update_min_max();
			},
			_ => {},
		}
	}
}

#[derive(Debug)]
pub enum Modus {
	SelectView,
	SelectDraw,
	Draw(u32),
	SelectCombine,
	Combine(u32),
	Spawn,
	Delete,
	View(u32, Option<render::Lines>),
}

// https://tildesites.bowdoin.edu/~ltoma/teaching/cs3250-CompGeom/spring17/Lectures/cg-hull3d.pdf
fn convex_hull(points: &[na::Point3<f32>], min_height: f32, state: &render::State) -> render::Lines {
	#[derive(Debug, Clone, Copy)]
	struct Point {
		idx: usize,
		pos: na::Point3<f32>,
	}

	impl PartialEq for Point {
		fn eq(&self, other: &Self) -> bool {
			self.idx == other.idx
		}
	}
	impl Eq for Point {}
	impl Hash for Point {
		fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
			self.idx.hash(state)
		}
	}

	let points = {
		let mut vec = Vec::new();
		for (idx, &p) in points.iter().enumerate() {
			if p.y >= min_height {
				vec.push(Point { idx, pos: p });
			}
		}
		vec
	};
	if points.len() < 10 {
		return render::Lines::new(state, &[0, 0]);
	}

	let mut first = points[0];
	for &p in points.iter() {
		if p.pos.y < first.pos.y {
			first = p;
		}
	}

	let mut best_value = f32::MAX;
	let mut second = None;
	for &p in points.iter() {
		if first.idx == p.idx {
			continue;
		}
		let v = p.pos - first.pos;
		let x = v.dot(&na::vector![1.0, 0.0, 0.0]);
		let y = v.dot(&na::vector![0.0, 1.0, 0.0]);
		let angle = y.atan2(x);
		assert!(angle >= 0.0);
		if angle < best_value {
			best_value = angle;
			second = Some(p);
		}
	}
	let second = second.unwrap();

	let mut iter = points
		.iter()
		.copied()
		.filter(|&p| p != first && p != second);
	let mut third = iter.next().unwrap();
	for p in iter {
		let out = (second.pos - first.pos)
			.normalize()
			.cross(&(third.pos - first.pos).normalize());
		if out.dot(&(p.pos - first.pos).normalize()) < 0.0 {
			third = p;
		}
	}

	let mut indices = Vec::new();
	indices.extend_from_slice(&[
		first.idx as u32,
		second.idx as u32,
		second.idx as u32,
		third.idx as u32,
		third.idx as u32,
		first.idx as u32,
	]);
	let mut edges = [(second, first), (third, second), (first, third)]
		.into_iter()
		.collect::<HashSet<_>>();

	while let Some(&(first, second)) = edges.iter().next() {
		edges.remove(&(first, second));

		let mut iter = points
			.iter()
			.copied()
			.filter(|&p| p != first && p != second);
		let mut third = iter.next().unwrap();
		for p in iter {
			let out = (second.pos - first.pos)
				.normalize()
				.cross(&(third.pos - first.pos).normalize());
			if out.dot(&(p.pos - first.pos).normalize()) < 0.0 {
				third = p;
			}
		}

		if edges.remove(&(third, first)).not() {
			edges.insert((first, third));
			indices.extend_from_slice(&[first.idx as u32, third.idx as u32]);
		}
		if edges.remove(&(second, third)).not() {
			edges.insert((third, second));
			indices.extend_from_slice(&[third.idx as u32, second.idx as u32]);
		}
	}

	render::Lines::new(state, &indices)
}

fn format_degrees(val: f64) -> String {
	let deg = val.to_degrees();
	let min = deg.fract() * if deg >= 0.0 { 60.0 } else { -60.0 };
	let deg = deg.trunc() as isize;
	let (min, sec) = (min.trunc() as isize, min.fract() * 60.0);
	format!("{:0>2}°{:0>2}'{:0>4.1}\"", deg, min, sec)
}
