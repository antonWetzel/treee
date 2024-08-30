use nalgebra as na;
use std::{collections::HashMap, ops::Not};

use crate::{
	calculations::{map_to_u32, CalculationProperties, Classification, SegmentData, SegmentSave},
	environment::{self, Saver},
	hull::{ConvexHull, Hull, IncludeMode, RadialBoundingVolume},
	program::{DisplaySettings, Event},
};

/// Special index for the deleted index.
pub const DELETED_INDEX: u32 = 0;

/// Unique ID based on the current source code location.
#[macro_export]
macro_rules! id {
	() => {
		(line!(), column!())
	};
}

/// State for the Interactive phase.
pub struct Interactive {
	pub segments: HashMap<u32, SegmentData>,
	pub deleted: SegmentData,
	sender: crossbeam::channel::Sender<Event>,

	pub modus: Modus,
	pub show_deleted: bool,
	draw_radius: f32,

	#[cfg(not(target_arch = "wasm32"))]
	pub source_location: String,
	world_offset: na::Point3<f64>,
}

/// Data to save and load interactive phase.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InteractiveSave {
	pub segments: HashMap<u32, SegmentData>,
	pub deleted: SegmentData,
	pub world_offset: na::Point3<f64>,
}

/// Default location (europe) to convert position to global coordinates.
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_LOCATION: &str = "+proj=utm\n+ellps=GRS80\n+zone=32";

impl SegmentData {
	/// First and second intersection with the bounding box.
	/// Returns `None` if the ray does not hit the bounding box.
	///
	/// Source: <https://tavianator.com/2011/ray_box.html>
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

	/// Distance to the first point intersection the ray.
	/// Returns `None` if the ray does not hit a point.
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

	/// Remove the points inside the sphere from the segment and add them to the target segment.
	/// Returns `true` if any point changes segment.
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
		let p = self.points.as_mut_slice();
		let c = self.classifications.as_mut_slice();

		// retain with multiple vecs
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

		changed
	}

	/// Change classification for every point inside the sphere.
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

	/// Update cashed information and data required for rendering.
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

	/// Update information for the viewed segment.
	fn update_info(&mut self, calc_curve: bool) -> CalculationProperties {
		self.info.update(
			&self.points,
			&self.classifications,
			self.min.y,
			self.max.y,
			calc_curve,
		)
	}

	#[cfg(not(target_arch = "wasm32"))]
	/// Update the world coordinates.
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
	/// Create a new Interactive with the segments.
	pub fn new(
		segments: HashMap<u32, SegmentData>,
		world_offset: na::Point3<f64>,
	) -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();
		let deleted = SegmentData::new(Vec::new());

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
		};

		(interactive, receiver)
	}

	/// Load a Interactive from a file.
	pub fn load(source: environment::Source) -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();

		let reader = source.reader();
		let save = bincode::deserialize_from::<_, InteractiveSave>(reader).unwrap();

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
		};

		(interactive, receiver)
	}

	/// Draw the UI
	pub fn ui(&mut self, ui: &mut egui::Ui) {
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
			environment::Saver::start("pointcloud.ipc", move |mut saver| {
				bincode::serialize_into(saver.inner(), &save).unwrap();
				saver.save();
			});
		}
		let enabled = matches!(self.modus, Modus::View(_)).not();

		ui.add_enabled_ui(enabled, |ui| {
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
		});

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
				if let Modus::View(ref view) = self.modus {
					let seg = self.segments.get_mut(&view.idx).unwrap();
					match proj4rs::Proj::from_proj_string(&self.source_location) {
						Ok(proj) => seg.update_location(self.world_offset, &proj),
						Err(err) => eprintln!("{}", err),
					}
				}
			};
		}
	}

	pub fn extra_ui(&mut self, ctx: &egui::Context, state: &render::State) {
		let Modus::View(view) = &mut self.modus else {
			return;
		};
		let mut close_view = false;

		egui::SidePanel::right("extra-panel")
			.resizable(false)
			.min_width(200.0)
			.show(ctx, |ui| {
				egui::ScrollArea::vertical().show(ui, |ui| {
					ui.add_sized(
						[ui.available_width(), 0.0],
						egui::Label::new(egui::RichText::new("Segment").heading()),
					);
					close_view = ui
						.add_sized([ui.available_width(), 0.0], egui::Button::new("Return"))
						.clicked();

					ui.separator();

					let segment = self.segments.get_mut(&view.idx).unwrap();

					ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Edit Points"));
					ui.radio_value(&mut view.modus, ViewModus::Delete, "Delete");
					ui.radio_value(&mut view.modus, ViewModus::Ground, "Ground");
					ui.radio_value(&mut view.modus, ViewModus::Trunk, "Trunk");
					ui.radio_value(&mut view.modus, ViewModus::Crown, "Crown");

					ui.separator();
					ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Display"));
					ui.radio_value(
						&mut view.display_modus,
						DisplayModus::Classification,
						"Classification",
					);
					ui.radio_value(&mut view.display_modus, DisplayModus::Curve, "Curve");
					ui.radio_value(
						&mut view.display_modus,
						DisplayModus::Expansion,
						"Expansion",
					);
					ui.radio_value(&mut view.display_modus, DisplayModus::Height, "Height");

					ui.separator();
					ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Hull"));
					if ui
						.add(egui::RadioButton::new(
							matches!(view.hull, Hull::None),
							"None",
						))
						.clicked()
					{
						view.hull = Hull::None;
					}
					if ui
						.add(egui::RadioButton::new(
							matches!(view.hull, Hull::Convex(_)),
							"Convex Hull",
						))
						.clicked()
					{
						view.hull = Hull::Convex(ConvexHull::new(
							&segment.points,
							&segment.classifications,
							IncludeMode::Crown,
							state,
						));
					}
					if ui
						.add(egui::RadioButton::new(
							matches!(view.hull, Hull::RadialBoundingVolume(_)),
							"Radial Bounding Volume",
						))
						.clicked()
					{
						view.hull = Hull::RadialBoundingVolume(RadialBoundingVolume::new(
							IncludeMode::All,
							&segment.points,
							&segment.classifications,
							8,
							8,
							state,
						));
					}
					view.hull.ui(ui, segment, state);

					ui.separator();
					if ui
						.add_sized(
							[ui.available_width(), 0.0],
							egui::Button::new("Update Curvature"),
						)
						.clicked()
					{
						view.calculations_properties = segment.update_info(true);
						view.display_data =
							DisplayData::new(state, segment, &view.calculations_properties);
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
						let seg = self.segments.get(&view.idx).unwrap();
						let points = seg.points.clone();
						let classifications = seg.classifications.clone();
						let calculations_properties = view.calculations_properties.clone();
						environment::Saver::start("points.ply", move |mut saver| {
							save_points(
								&mut saver,
								&points,
								&classifications,
								&calculations_properties,
								|_| true,
							)
							.unwrap();
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
						let seg = self.segments.get(&view.idx).unwrap();
						let save = SegmentSave {
							info: seg.info,
							min: seg.min,
							max: seg.max,
							offset: self.world_offset,
							longitude: seg.coords.map(|c| c.0.to_degrees()),
							latitude: seg.coords.map(|c| c.1.to_degrees()),
						};
						environment::Saver::start("segment.json", move |mut saver| {
							serde_json::to_writer_pretty(saver.inner(), &save).unwrap();
							saver.save();
						});
					}

					for (name, file, classification) in [
						("Crown", "crown.ply", Classification::Crown),
						("Trunk", "trunk.ply", Classification::Trunk),
						("Ground", "ground.ply", Classification::Ground),
					] {
						if ui
							.add_sized([ui.available_width(), 0.0], egui::Button::new(name))
							.clicked()
						{
							let seg = self.segments.get(&view.idx).unwrap();
							let points = seg.points.clone();
							let classifications = seg.classifications.clone();
							let calculations_properties = view.calculations_properties.clone();
							environment::Saver::start(file, move |mut saver| {
								save_points(
									&mut saver,
									&points,
									&classifications,
									&calculations_properties,
									|c| c == classification,
								)
								.unwrap();
								saver.save();
							})
						}
					}
				});
			});
		if close_view {
			self.modus = Modus::SelectView;
		}
	}

	/// Get the first segment and distance hit by the ray.
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

	/// Handle mouse click.
	pub fn click(
		&mut self,
		start: na::Point3<f32>,
		direction: na::Vector3<f32>,
		display_settings: &DisplaySettings,
		state: &render::State,
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
				let calculations_properties = seg.update_info(true);

				#[cfg(not(target_arch = "wasm32"))]
				match proj4rs::Proj::from_proj_string(&self.source_location) {
					Ok(proj) => seg.update_location(self.world_offset, &proj),
					Err(err) => eprintln!("{}", err),
				}

				let display_data = DisplayData::new(state, seg, &calculations_properties);

				self.modus = Modus::View(View {
					idx,
					hull: Hull::None,
					display_modus: DisplayModus::Classification,
					modus: ViewModus::Delete,
					display_data,
					calculations_properties,
					cloud: render::PointCloud::new(state, &seg.points),
				})
			},

			Modus::View { .. } => {},
		}
	}

	/// Handle mouse drag.
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
				if self.show_deleted && self.deleted.remove(hit, self.draw_radius, &mut target) {
					self.deleted.changed(DELETED_INDEX, &self.sender);
					changed = true;
				}

				if changed {
					target.changed(idx, &self.sender);
				}
				self.segments.insert(idx, target);
				for empty in empty {
					self.segments.remove(&empty);
				}
			},
			Modus::View(ref mut view) => {
				let seg = self.segments.get_mut(&view.idx).unwrap();
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

				let mut changed = false;

				if self.show_deleted
					&& view.modus != ViewModus::Delete
					&& self.deleted.remove(hit, self.draw_radius, seg)
				{
					self.deleted.changed(DELETED_INDEX, &self.sender);
					changed = true;
				}

				changed |= match view.modus {
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
					seg.changed(view.idx, &self.sender);
					view.calculations_properties = seg.update_info(false);
					view.display_data = DisplayData::new(state, seg, &view.calculations_properties);
					view.cloud = render::PointCloud::new(state, &seg.points);
					view.hull.update(seg, state);
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

/// Current Modus for the Interactive phase.
#[derive(Debug)]
pub enum Modus {
	SelectView,
	SelectDraw,
	Draw(u32),
	SelectCombine,
	Combine(u32),
	Spawn,
	Delete,
	View(View),
}

/// Selected segment to view.
#[derive(Debug)]
pub struct View {
	pub idx: u32,
	pub modus: ViewModus,
	pub display_modus: DisplayModus,
	pub cloud: render::PointCloud,
	pub display_data: DisplayData,
	pub calculations_properties: CalculationProperties,
	pub hull: Hull,
}

/// Display data for selected segment.
#[derive(Debug)]
pub struct DisplayData {
	pub classification: render::PointCloudProperty,
	pub curve: render::PointCloudProperty,
	pub expansion: render::PointCloudProperty,
	pub height: render::PointCloudProperty,
}

/// Display modus to render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayModus {
	Classification,
	Curve,
	Expansion,
	Height,
}

impl DisplayData {
	/// Create DisplayData from values.
	pub fn new(state: &render::State, seg: &SegmentData, calc: &CalculationProperties) -> Self {
		let max_expansion = calc
			.expansion
			.iter()
			.copied()
			.max_by(|a, b| a.total_cmp(b))
			.unwrap_or_default();
		let expansion = calc
			.expansion
			.iter()
			.copied()
			.map(|e| map_to_u32(e / max_expansion))
			.collect::<Vec<_>>();

		let curve = calc
			.curve
			.iter()
			.copied()
			.map(map_to_u32)
			.collect::<Vec<_>>();
		let height = calc
			.height
			.iter()
			.copied()
			.map(map_to_u32)
			.collect::<Vec<_>>();

		let classification = seg
			.classifications
			.iter()
			.map(|c| match c {
				Classification::Ground => u32::MAX / 8,
				Classification::Trunk => u32::MAX / 8 * 3,
				Classification::Crown => u32::MAX / 8 * 6,
			})
			.collect::<Vec<_>>();

		Self {
			classification: render::PointCloudProperty::new(state, &classification),
			curve: render::PointCloudProperty::new(state, &curve),
			expansion: render::PointCloudProperty::new(state, &expansion),
			height: render::PointCloudProperty::new(state, &height),
		}
	}
}

/// Edit modus for selected segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewModus {
	Delete,
	Ground,
	Trunk,
	Crown,
}

/// Format radians as degrees, minutes and seconds.
fn format_degrees(val: f64) -> String {
	let deg = val.to_degrees();
	let min = deg.fract() * if deg >= 0.0 { 60.0 } else { -60.0 };
	let deg = deg.trunc() as isize;
	let (min, sec) = (min.trunc() as isize, min.fract() * 60.0);
	format!("{:0>2}°{:0>2}'{:0>4.1}\"", deg, min, sec)
}

/// Save points as `.ply`.
pub fn save_points(
	saver: &mut Saver,
	points: &[na::Point3<f32>],
	classifications: &[Classification],
	calculations_properties: &CalculationProperties,
	valid: impl Fn(Classification) -> bool,
) -> Result<(), std::io::Error> {
	use std::io::Write;

	let count = classifications.iter().filter(|&&c| valid(c)).count();
	let mut writer = saver.inner();
	writeln!(writer, "ply")?;
	writeln!(writer, "format ascii 1.0")?;
	writeln!(writer, "element vertex {}", count)?;
	writeln!(writer, "property float x")?;
	writeln!(writer, "property float y")?;
	writeln!(writer, "property float z")?;
	writeln!(writer, "property float expansion")?;
	writeln!(writer, "property float height")?;
	writeln!(writer, "property float curve")?;
	writeln!(writer, "end_header")?;
	for (idx, p) in points.iter().enumerate() {
		if valid(classifications[idx]).not() {
			continue;
		}
		writeln!(
			writer,
			"{} {} {} {} {} {}",
			p.x,
			-p.z,
			p.y,
			calculations_properties.expansion[idx],
			calculations_properties.height[idx],
			calculations_properties.curve[idx]
		)?;
	}

	Ok(())
}
