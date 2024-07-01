use nalgebra as na;
use std::{collections::HashMap, ops::Not, sync::Arc};

use crate::{
	calculations::{DisplayModus, Segment, SegmentRender},
	program::Event,
};

pub struct Interactive {
	state: Arc<render::State>,
	pub segments: HashMap<usize, Segment>,
	pub deleted: Segment,
	pub display: DisplayModus,

	pub modus: Modus,
	pub show_deleted: bool,
	draw_radius: f32,
}

impl Segment {
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

	pub fn update_render(&mut self, idx: usize, state: &render::State) {
		self.render = SegmentRender::new(&self.points, idx, self.info, state);
	}

	pub fn height(&self) -> f32 {
		self.max.y - self.min.y
	}
}

impl Interactive {
	pub fn new(
		segments: HashMap<usize, Segment>,
		state: Arc<render::State>,
		display: DisplayModus,
	) -> (Self, crossbeam::channel::Receiver<Event>) {
		let (_sender, receiver) = crossbeam::channel::unbounded();
		let deleted = Segment::new(Vec::new(), 0, &state);

		let interactive = Self {
			state,
			segments,
			modus: Modus::SelectView,
			deleted,
			draw_radius: 0.5,
			display,
			show_deleted: false,
		};

		(interactive, receiver)
	}

	pub fn ui(&mut self, ui: &mut egui::Ui) {
		if ui
			.radio(matches!(self.display, DisplayModus::Solid), "Segment")
			.clicked()
		{
			self.display = DisplayModus::Solid;
		}
		if ui
			.radio(
				matches!(self.display, DisplayModus::Property),
				"Classification",
			)
			.clicked()
		{
			self.display = DisplayModus::Property;
		}
		ui.separator();
		if let Modus::View(_) = self.modus {
			if ui.button("Return").clicked() {
				self.modus = Modus::SelectView;
			}
		} else {
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
		ui.checkbox(&mut self.show_deleted, "Show Deleted");

		ui.separator();
		ui.label("remove radius");
		ui.add(
			egui::Slider::new(&mut self.draw_radius, 0.1..=10.0)
				.logarithmic(true)
				.suffix("m"),
		);
		ui.separator();

		match self.modus {
			Modus::SelectView => {},
			Modus::Spawn => {},
			Modus::SelectDraw | Modus::Draw(_) => {},
			Modus::SelectCombine | Modus::Combine(_) => {},
			Modus::Delete => {},
			Modus::View(idx) => {
				let segment = self.segments.get_mut(&idx).unwrap();
				let mut changed = false;
				let mut rel_ground = segment.info.ground_sep - segment.min.y;
				ui.label("trunk start");
				if ui
					.add(egui::Slider::new(&mut rel_ground, 0.0..=segment.height()).suffix("m"))
					.changed()
				{
					changed = true;
					segment.info.ground_sep = segment.min.y + rel_ground;
					segment.info.crown_sep = segment.info.crown_sep.max(segment.info.ground_sep);
				}
				let mut rel_crown = segment.info.crown_sep - segment.min.y;
				ui.label("crown start");
				if ui
					.add(egui::Slider::new(&mut rel_crown, 0.0..=segment.height()).suffix("m"))
					.changed()
				{
					changed = true;
					segment.info.crown_sep = segment.min.y + rel_crown;
					segment.info.ground_sep = segment.info.ground_sep.min(segment.info.crown_sep);
				}
				if changed {
					segment
						.info
						.redo_diameters(&segment.points, segment.min.y, segment.max.y);
					segment.update_render(idx, &self.state);
				}
				ui.separator();
				ui.label("trunk diameter");
				ui.label(format!("{}m", segment.info.trunk_diameter));
				ui.label("crown diameter");
				ui.label(format!("{}m", segment.info.crown_diameter));
				ui.separator();

				if ui.button("Export (todo)").clicked() {
					println!("todo");
				}
			},
		};
	}

	fn select(&self, start: na::Point3<f32>, direction: na::Vector3<f32>) -> Option<(usize, f32)> {
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
				let mut empty: Vec<usize> = Vec::new();
				for (&other, segment) in self.segments.iter_mut() {
					let seg_changed = segment.remove(hit, self.draw_radius, &mut points);
					if segment.points.is_empty() {
						empty.push(other);
					} else if seg_changed {
						segment.update_render(other, &self.state);
					}
				}
				for empty in empty {
					self.segments.remove(&empty);
				}
				if points.is_empty() {
					return;
				}

				let mut idx = rand::random();
				while self.segments.contains_key(&idx) {
					idx = rand::random();
				}
				let segment = Segment::new(points, idx, &self.state);
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
				self.modus = Modus::View(idx);
			},

			Modus::View(_) => {},
		}
	}

	pub fn drag(&mut self, start: na::Point3<f32>, direction: na::Vector3<f32>) {
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
						segment.update_render(other, &self.state);
						segment.update_min_max();
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
					self.deleted.update_min_max();
					self.deleted.update_render(0, &self.state);
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
						if self.deleted.raycast_distance(start, direction).is_none() {
							return None;
						}
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
						segment.update_render(other, &self.state);
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
						self.deleted.update_render(0, &self.state);
						self.deleted.update_min_max();
						changed = true;
					}
				}

				if changed {
					target.update_render(idx, &self.state);
					target.update_min_max();
				}
				self.segments.insert(idx, target);
				for empty in empty {
					self.segments.remove(&empty);
				}
			},
			Modus::View(idx) => {
				let seg = self.segments.get_mut(&idx).unwrap();
				let Some(distance) = seg.exact_distance(start, direction) else {
					return;
				};
				let hit = start + direction * distance;
				let target = &mut self.deleted;
				let mut changed = false;
				if seg.remove(hit, self.draw_radius, &mut target.points) {
					seg.update_render(idx, &self.state);
					seg.update_min_max();
					changed = true;
				}
				if changed {
					target.update_min_max();
					target.update_render(0, &self.state);
				}
			},
			Modus::Combine(idx) => {
				let Some((other, _)) = self.select(start, direction) else {
					return;
				};
				if other == idx {
					return;
				}
				let mut other = self.segments.remove(&other).unwrap();
				let target = self.segments.get_mut(&idx).unwrap();
				target.points.append(&mut other.points);
				target.update_render(idx, &self.state);
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
	Draw(usize),
	SelectCombine,
	Combine(usize),
	Spawn,
	Delete,
	View(usize),
}
