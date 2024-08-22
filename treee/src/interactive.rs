use nalgebra as na;
use std::{
	collections::{HashMap, HashSet},
	hash::Hash,
	ops::Not,
};

use crate::{
	calculations::{Classification, SegmentData, SegmentSave},
	environment::{self, Saver},
	program::{DisplaySettings, Event},
};

pub const DELETED_INDEX: u32 = 0;

macro_rules! id {
	() => {
		(line!(), column!())
	};
}

pub struct Interactive {
	pub segments: HashMap<u32, SegmentData>,
	pub deleted: SegmentData,
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
	pub fn raycast_distance(
		&self,
		start: na::Point3<f32>,
		direction: na::Vector3<f32>,
	) -> Option<(f32, f32)> {
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

	pub fn exact_distance(
		&self,
		start: na::Point3<f32>,
		direction: na::Vector3<f32>,
		display_settings: &DisplaySettings,
	) -> Option<f32> {
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
			if distance > display_settings.point_cloud_environment.scale {
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

	pub fn remove(&mut self, center: na::Point3<f32>, radius: f32, target: &mut Self) -> bool {
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

		let len = self.points.len();
		let mut del = 0;
		{
			let p = self.points.as_mut_slice();
			let c = self.classifications.as_mut_slice();

			for i in 0..len {
				if (p[i] - center).norm_squared() <= r2 {
					del += 1;
					target.points.push(p[i]);
					target.classifications.push(c[i]);
					changed = true;
				} else {
					p.swap(i - del, i);
					c.swap(i - del, i);
				}
			}
			self.points.truncate(len - del);
			self.classifications.truncate(len - del);
		}
		changed
	}

	pub fn change_classification(
		&mut self,
		center: na::Point3<f32>,
		radius: f32,
		classification: Classification,
	) -> bool {
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
		self.points
			.iter()
			.zip(self.classifications.as_mut_slice())
			.for_each(|(&p, c)| {
				if (p - center).norm_squared() > r2 {
					return;
				}
				*c = classification;
				changed = true;
			});
		changed
	}

	fn changed(&mut self, idx: u32, sender: &crossbeam::channel::Sender<Event>) {
		if self.points.is_empty() {
			return;
		}

		(self.min, self.max) = (self.points[0], self.points[0]);
		let (mut trunk_min, mut trunk_max) = (f32::MAX, f32::MIN);
		let (mut crown_min, mut crown_max) = (f32::MAX, f32::MIN);

		for (&p, &c) in self.points.iter().zip(&self.classifications) {
			for dim in 0..3 {
				self.min[dim] = self.min[dim].min(p[dim]);
				self.max[dim] = self.max[dim].max(p[dim]);
			}
			match c {
				Classification::Ground => {},
				Classification::Trunk => {
					trunk_min = trunk_min.min(p.y);
					trunk_max = trunk_max.max(p.y);
				},
				Classification::Crown => {
					crown_min = crown_min.min(p.y);
					crown_max = crown_max.max(p.y);
				},
			}
		}
		if trunk_min > trunk_max {
			self.info.trunk_height = 0.0;
		} else {
			self.info.trunk_height = trunk_max - trunk_min;
			self.info.ground_sep = trunk_min;
		}
		if crown_min > crown_max {
			self.info.crown_height = 0.0;
		} else {
			self.info.crown_height = crown_max - crown_min;
			self.info.crown_sep = crown_min;
		}
		self.update_render(idx, sender)
	}

	fn redo_diameters(&mut self) {
		self.info
			.redo_diameters(&self.points, &self.classifications, self.min.y, self.max.y);
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
}

impl Interactive {
	pub fn new(
		segments: HashMap<u32, SegmentData>,
		state: &render::State,
		world_offset: na::Point3<f64>,
	) -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();
		let deleted = SegmentData::new(Vec::new());
		let white_lookup =
			render::Lookup::new_png(state, include_bytes!("../assets/white.png"), u32::MAX);

		let interactive = Self {
			segments,
			modus: Modus::SelectView,
			deleted,
			draw_radius: 0.5,
			sender,
			show_deleted: false,
			#[cfg(not(target_arch = "wasm32"))]
			source_location: DEFAULT_LOCATION.into(),
			world_offset,
			white_lookup,
		};

		(interactive, receiver)
	}

	pub fn load(
		source: environment::Source,
		state: &render::State,
	) -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();
		_ = sender.send(Event::Lookup {
			bytes: include_bytes!("../assets/grad_turbo.png"),
			max: u32::MAX,
		});

		let reader = source.reader();
		let save = bincode::deserialize_from::<_, InteractiveSave>(reader).unwrap();
		let white_lookup =
			render::Lookup::new_png(&state, include_bytes!("../assets/white.png"), u32::MAX);

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
			environment::Saver::new("pointcloud.ipc", move |mut saver| {
				bincode::serialize_into(saver.inner(), &save).unwrap();
				saver.save();
			});
		}

		if let Modus::View { .. } = self.modus {
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
		egui::Grid::new(id!()).num_columns(2).show(ui, |ui| {
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
				if let Modus::View { idx, .. } = self.modus {
					let seg = self.segments.get_mut(&idx).unwrap();
					match proj4rs::Proj::from_proj_string(&self.source_location) {
						Ok(proj) => seg.update_location(self.world_offset, &proj),
						Err(err) => eprintln!("{}", err),
					}
				}
			};
		}

		if let Modus::View { .. } = self.modus {
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
			Modus::View { idx, ref mut convex_hull, ref mut modus } => {
				let segment = self.segments.get_mut(&idx).unwrap();

				ui.separator();
				ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Modus"));
				ui.radio_value(modus, ViewModus::Delete, "Delete");
				ui.radio_value(modus, ViewModus::Ground, "Ground");
				ui.radio_value(modus, ViewModus::Trunk, "Trunk");
				ui.radio_value(modus, ViewModus::Crown, "Crown");

				ui.separator();
				let mut render_hull = convex_hull.is_some();
				if ui.checkbox(&mut render_hull, "Convex Hull").changed() {
					*convex_hull = if render_hull {
						Some(ConvexHull::new(
							&segment.points,
							&segment.classifications,
							state,
						))
					} else {
						None
					};
				}

				ui.separator();
				ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Trunk"));
				egui::Grid::new(id!()).num_columns(2).show(ui, |ui| {
					ui.label("Height");
					ui.label(format!("{:.2}m", segment.info.trunk_height));
					ui.end_row();

					ui.label("Diameter");
					ui.label(format!("{:.2}m", segment.info.trunk_diameter));
					ui.end_row();
				});

				ui.separator();
				ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Crown"));
				egui::Grid::new(id!()).num_columns(2).show(ui, |ui| {
					ui.label("Height");
					ui.label(format!("{:.2}m", segment.info.crown_height));
					ui.end_row();

					ui.label("Diameter");
					ui.label(format!("{:.2}m", segment.info.crown_diameter));
					ui.end_row();
				});

				if let Some((long, lat)) = segment.coords {
					ui.separator();
					ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Coordinates"));
					egui::Grid::new(id!()).num_columns(2).show(ui, |ui| {
						ui.label("Lat");
						ui.label(format_degrees(lat));
						ui.end_row();

						ui.label("Long");
						ui.label(format_degrees(long));
						ui.end_row();
					});
				}

				ui.separator();
				ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Export"));
				if ui
					.add_sized([ui.available_width(), 0.0], egui::Button::new("Points"))
					.clicked()
				{
					let points = self.segments.get(&idx).unwrap().points.clone();
					environment::Saver::new("points.ply", move |mut saver| {
						save_points(&mut saver, &points).unwrap();
						saver.save();
					})
				}
				if ui
					.add_sized(
						[ui.available_width(), 0.0],
						egui::Button::new("Information"),
					)
					.clicked()
				{
					let seg = self.segments.get(&idx).unwrap();
					let save = SegmentSave {
						info: seg.info,
						min: seg.min,
						max: seg.max,
						offset: self.world_offset,
						longitude: seg.coords.map(|c| c.0.to_degrees()),
						latitude: seg.coords.map(|c| c.1.to_degrees()),
					};
					environment::Saver::new("segment.json", move |mut saver| {
						serde_json::to_writer_pretty(saver.inner(), &save).unwrap();
						saver.save();
					});
				}
				if let Some(hull) = convex_hull {
					if ui
						.add_sized(
							[ui.available_width(), 0.0],
							egui::Button::new("Convex Hull"),
						)
						.clicked()
					{
						let points = self.segments.get(&idx).unwrap().points.clone();
						let faces = hull.faces.clone();
						environment::Saver::new("convex_hull.ply", move |mut saver| {
							ConvexHull::save(&mut saver, &points, &faces).unwrap();
							saver.save();
						})
					}
				}
			},
		};
	}

	fn select(
		&self,
		start: na::Point3<f32>,
		direction: na::Vector3<f32>,
		display_settings: &DisplaySettings,
	) -> Option<(u32, f32)> {
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
			let Some(d) = self.segments[&idx].exact_distance(start, direction, display_settings)
			else {
				continue;
			};
			if d < distance {
				distance = d;
				best = Some(idx);
			}
		}
		best.map(|idx| (idx, distance))
	}

	pub fn click(
		&mut self,
		start: na::Point3<f32>,
		direction: na::Vector3<f32>,
		display_settings: &DisplaySettings,
	) {
		match &mut self.modus {
			Modus::SelectDraw | Modus::Draw(_) => {
				self.modus = if let Some((idx, _)) = self.select(start, direction, display_settings)
				{
					Modus::Draw(idx)
				} else {
					Modus::SelectDraw
				};
			},
			Modus::SelectCombine | Modus::Combine(_) => {
				self.modus = if let Some((idx, _)) = self.select(start, direction, display_settings)
				{
					Modus::Combine(idx)
				} else {
					Modus::SelectCombine
				};
			},
			Modus::Spawn => {
				let Some((_, distance)) = self.select(start, direction, display_settings) else {
					return;
				};
				let hit = start + direction * distance;
				let mut new_segment = SegmentData::new(Vec::new());
				let mut empty = Vec::new();
				for (&other, segment) in self.segments.iter_mut() {
					let seg_changed = segment.remove(hit, self.draw_radius, &mut new_segment);
					if segment.points.is_empty() {
						empty.push(other);
					} else if seg_changed {
						segment.update_render(other, &self.sender);
					}
				}
				for empty in empty {
					_ = self.sender.send(Event::RemovePointCloud(empty));
					self.segments.remove(&empty);
				}
				if new_segment.points.is_empty() {
					return;
				}

				let mut idx = rand::random();
				while idx == DELETED_INDEX || self.segments.contains_key(&idx) {
					idx = rand::random();
				}
				new_segment.update_render(idx, &self.sender);
				self.segments.insert(idx, new_segment);
				self.modus = Modus::Draw(idx);
			},
			Modus::Delete => {},

			Modus::SelectView => {
				let Some((idx, _)) = self.select(start, direction, display_settings) else {
					return;
				};
				let seg = self.segments.get_mut(&idx).unwrap();
				seg.redo_diameters();

				#[cfg(not(target_arch = "wasm32"))]
				match proj4rs::Proj::from_proj_string(&self.source_location) {
					Ok(proj) => seg.update_location(self.world_offset, &proj),
					Err(err) => eprintln!("{}", err),
				}

				self.modus = Modus::View {
					idx,
					convex_hull: None,
					modus: ViewModus::Delete,
				}
			},

			Modus::View { .. } => {},
		}
	}

	pub fn drag(
		&mut self,
		start: na::Point3<f32>,
		direction: na::Vector3<f32>,
		state: &render::State,
		display_settings: &DisplaySettings,
	) {
		match self.modus {
			Modus::Delete => {
				let Some((_, distance)) = self.select(start, direction, display_settings) else {
					return;
				};
				let hit = start + direction * distance;
				let mut changed = false;
				let mut empty = Vec::new();
				for (&other, segment) in self.segments.iter_mut() {
					if segment.remove(hit, self.draw_radius, &mut self.deleted) {
						segment.changed(other, &self.sender);
						changed = true;
					}
					if segment.points.is_empty() {
						empty.push(other);
					}
				}
				for empty in empty {
					self.segments.remove(&empty);
				}
				if changed {
					self.deleted.changed(DELETED_INDEX, &self.sender);
				}
			},
			Modus::Draw(idx) => {
				let Some(distance) = self
					.select(start, direction, display_settings)
					.map(|(_, distance)| distance)
					.or_else(|| {
						if self.show_deleted.not() {
							return None;
						}
						self.deleted.raycast_distance(start, direction)?;
						self.deleted
							.exact_distance(start, direction, display_settings)
					})
				else {
					return;
				};
				let hit = start + direction * distance;
				let mut target = self.segments.remove(&idx).unwrap();
				let mut changed = false;
				let mut empty = Vec::new();
				for (&other, segment) in self.segments.iter_mut() {
					if segment.remove(hit, self.draw_radius, &mut target) {
						segment.changed(other, &self.sender);
						changed = true;
					}
					if segment.points.is_empty() {
						empty.push(other);
					}
				}
				if self.show_deleted {
					if self.deleted.remove(hit, self.draw_radius, &mut target) {
						self.deleted.changed(DELETED_INDEX, &self.sender);
						changed = true;
					}
				}

				if changed {
					target.changed(idx, &self.sender);
				}
				self.segments.insert(idx, target);
				for empty in empty {
					self.segments.remove(&empty);
				}
			},
			Modus::View { idx, convex_hull: ref mut hull, modus } => {
				let seg = self.segments.get_mut(&idx).unwrap();
				let mut distance = seg.exact_distance(start, direction, display_settings);

				if self.show_deleted {
					if let Some(del_distance) = self
						.deleted
						.exact_distance(start, direction, display_settings)
						.or_else(|| seg.exact_distance(start, direction, display_settings))
					{
						if distance.map(|d| d > del_distance).unwrap_or(true) {
							distance = Some(del_distance);
						}
					};
				}
				let Some(distance) = distance else {
					return;
				};
				let hit = start + direction * distance;

				let changed = match modus {
					ViewModus::Delete => {
						if seg.remove(hit, self.draw_radius, &mut self.deleted) {
							self.deleted.changed(DELETED_INDEX, &self.sender);
							true
						} else {
							false
						}
					},
					ViewModus::Ground => {
						seg.change_classification(hit, self.draw_radius, Classification::Ground)
					},
					ViewModus::Trunk => {
						seg.change_classification(hit, self.draw_radius, Classification::Trunk)
					},
					ViewModus::Crown => {
						seg.change_classification(hit, self.draw_radius, Classification::Crown)
					},
				};
				if changed {
					seg.changed(idx, &self.sender);
					seg.redo_diameters();
					if let Some(hull) = hull {
						*hull = ConvexHull::new(&seg.points, &seg.classifications, state);
					}
				}
			},
			Modus::Combine(idx) => {
				let Some((other, _)) = self.select(start, direction, display_settings) else {
					return;
				};
				if other == idx {
					return;
				}
				_ = self.sender.send(Event::RemovePointCloud(other));
				let mut other = self.segments.remove(&other).unwrap();
				let target = self.segments.get_mut(&idx).unwrap();
				target.points.append(&mut other.points);
				target.changed(idx, &self.sender);
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
	View {
		idx: u32,
		modus: ViewModus,
		convex_hull: Option<ConvexHull>,
	},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewModus {
	Delete,
	Ground,
	Trunk,
	Crown,
}

#[derive(Debug)]
pub struct ConvexHull {
	faces: Vec<[u32; 3]>,
	pub lines: render::Lines,
}

impl ConvexHull {
	// https://tildesites.bowdoin.edu/~ltoma/teaching/cs3250-CompGeom/spring17/Lectures/cg-hull3d.pdf
	fn new(
		points: &[na::Point3<f32>],
		classifications: &[Classification],
		state: &render::State,
	) -> Self {
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

		let points = points
			.iter()
			.copied()
			.enumerate()
			.zip(classifications)
			.filter_map(|((idx, pos), &c)| {
				(c == Classification::Crown).then_some(Point { idx, pos })
			})
			.collect::<Vec<_>>();

		if points.len() < 10 {
			return Self {
				faces: Vec::new(),
				lines: render::Lines::new(state, &[0, 0]),
			};
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
		let mut faces = Vec::new();
		indices.extend_from_slice(&[
			first.idx as u32,
			second.idx as u32,
			second.idx as u32,
			third.idx as u32,
			third.idx as u32,
			first.idx as u32,
		]);
		faces.push([first.idx as u32, second.idx as u32, third.idx as u32]);

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

			faces.push([first.idx as u32, second.idx as u32, third.idx as u32]);

			if edges.remove(&(third, first)).not() {
				edges.insert((first, third));
				indices.extend_from_slice(&[first.idx as u32, third.idx as u32]);
			}
			if edges.remove(&(second, third)).not() {
				edges.insert((third, second));
				indices.extend_from_slice(&[third.idx as u32, second.idx as u32]);
			}
		}

		Self {
			lines: render::Lines::new(state, &indices),
			faces,
		}
	}

	pub fn save(
		saver: &mut Saver,
		points: &[na::Point3<f32>],
		faces: &[[u32; 3]],
	) -> Result<(), std::io::Error> {
		use std::io::Write;

		let mut mapping = HashMap::new();
		let mut used_points = Vec::new();
		for &face in faces {
			for idx in face.into_iter() {
				if mapping.contains_key(&idx) {
					continue;
				}
				mapping.insert(idx, used_points.len());
				used_points.push(idx);
			}
		}

		let mut writer = saver.inner();
		writeln!(writer, "ply")?;
		writeln!(writer, "format ascii 1.0")?;
		writeln!(writer, "element vertex {}", used_points.len())?;
		writeln!(writer, "property float x")?;
		writeln!(writer, "property float y")?;
		writeln!(writer, "property float z")?;
		writeln!(writer, "element face {}", faces.len())?;
		writeln!(writer, "property list uchar uint vertex_indices")?;
		writeln!(writer, "end_header")?;
		for idx in used_points {
			let p = points[idx as usize];
			writeln!(writer, "{} {} {}", p.x, -p.z, p.y)?;
		}
		for face in faces {
			writeln!(
				writer,
				"3 {} {} {}",
				mapping[&face[0]], mapping[&face[2]], mapping[&face[1]]
			)?;
		}
		Ok(())
	}
}

fn format_degrees(val: f64) -> String {
	let deg = val.to_degrees();
	let min = deg.fract() * if deg >= 0.0 { 60.0 } else { -60.0 };
	let deg = deg.trunc() as isize;
	let (min, sec) = (min.trunc() as isize, min.fract() * 60.0);
	format!("{:0>2}°{:0>2}'{:0>4.1}\"", deg, min, sec)
}

pub fn save_points(saver: &mut Saver, points: &[na::Point3<f32>]) -> Result<(), std::io::Error> {
	use std::io::Write;

	let mut writer = saver.inner();
	writeln!(writer, "ply")?;
	writeln!(writer, "format ascii 1.0")?;
	writeln!(writer, "element vertex {}", points.len())?;
	writeln!(writer, "property float x")?;
	writeln!(writer, "property float y")?;
	writeln!(writer, "property float z")?;
	writeln!(writer, "end_header")?;
	for &p in points {
		writeln!(writer, "{} {} {}", p.x, -p.z, p.y)?;
	}
	Ok(())
}
