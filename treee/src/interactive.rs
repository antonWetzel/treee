use nalgebra as na;
use render::Lookup;
use std::{collections::HashMap, ops::Not, sync::Arc};

use dashmap::DashMap;

use crate::{calculations::SegmentInformation, program::Event};

pub struct Interactive {
	state: Arc<render::State>,
	segments: HashMap<usize, Segment>,
	pub point_clouds: HashMap<usize, (render::PointCloud, render::PointCloudProperty)>,

	pub modus: Modus,
	draw_radius: f32,
}

struct Segment {
	points: Vec<na::Point3<f32>>,
	min: na::Point3<f32>,
	max: na::Point3<f32>,
}

impl Segment {
	pub fn empty() -> Self {
		Self {
			points: Vec::new(),
			min: na::point![0.0, 0.0, 0.0],
			max: na::point![0.0, 0.0, 0.0],
		}
	}

	pub fn new(points: Vec<na::Point3<f32>>) -> Self {
		let (mut min, mut max) = (points[0], points[0]);
		for &p in points.iter() {
			for dim in 0..3 {
				min[dim] = min[dim].min(p[dim]);
				max[dim] = max[dim].max(p[dim]);
			}
		}

		Self { points, min, max }
	}

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

	pub fn remove(&mut self, center: na::Point3<f32>, radius: f32, mut target: Option<&mut Self>) -> bool {
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
			if let Some(target) = &mut target {
				target.add_point(p);
			}
			changed = true;
			false
		});

		if changed && self.points.is_empty().not() {
			self.update_min_max();
		}
		changed
	}

	fn update_min_max(&mut self) {
		(self.min, self.max) = (self.points[0], self.points[0]);
		for &p in self.points.iter() {
			for dim in 0..3 {
				self.min[dim] = self.min[dim].min(p[dim]);
				self.max[dim] = self.max[dim].max(p[dim]);
			}
		}
	}

	fn add_point(&mut self, p: na::Point3<f32>) {
		self.points.push(p);
		for dim in 0..3 {
			self.min[dim] = self.min[dim].min(p[dim]);
			self.max[dim] = self.max[dim].max(p[dim]);
		}
	}

	pub fn render_data(&self, state: &render::State, index: usize) -> (render::PointCloud, render::PointCloudProperty) {
		let point_cloud = render::PointCloud::new(state, &self.points);
		let property = render::PointCloudProperty::new(state, &vec![index as u32; self.points.len()]);
		(point_cloud, property)
	}
}

impl Interactive {
	pub fn new(
		segments: DashMap<usize, Vec<na::Point3<f32>>>,
		state: Arc<render::State>,
	) -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();

		let mut seg = HashMap::new();
		let mut clouds = HashMap::new();
		for (_, segment) in segments.into_iter() {
			if segment.is_empty() {
				continue;
			}

			let mut idx = rand::random();
			while seg.contains_key(&idx) {
				idx = rand::random();
			}
			let point_cloud = render::PointCloud::new(&state, &segment);
			let property = render::PointCloudProperty::new(&state, &vec![idx as u32; segment.len()]);
			seg.insert(idx, Segment::new(segment));
			clouds.insert(idx, (point_cloud, property));
		}

		sender
			.send(Event::Lookup(Lookup::new_png(
				&state,
				include_bytes!("../../viewer/assets/grad_turbo.png"),
				u32::MAX,
			)))
			.unwrap();

		let interactive = Self {
			state,
			segments: seg,
			point_clouds: clouds,
			modus: Modus::SelectView,
			draw_radius: 0.5,
		};
		(interactive, receiver)
	}

	pub fn ui(&mut self, ui: &mut egui::Ui) -> InteractiveResponse {
		let response = InteractiveResponse::None;

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

		match &mut self.modus {
			Modus::SelectView => {},
			Modus::Spawn => {},
			Modus::SelectDraw | Modus::Draw(_) => {
				ui.label("Radius");
				ui.add(
					egui::Slider::new(&mut self.draw_radius, 0.1..=10.0)
						.logarithmic(true)
						.suffix("m"),
				);
			},
			Modus::Delete => {
				ui.label("Radius");
				ui.add(
					egui::Slider::new(&mut self.draw_radius, 0.1..=10.0)
						.logarithmic(true)
						.suffix("m"),
				);
			},
			Modus::View(view) => {
				let mut changed = false;
				let mut rel_ground = view.info.ground_sep - view.min;
				if ui
					.add(egui::Slider::new(&mut rel_ground, 0.0..=view.height).suffix("m"))
					.changed()
				{
					changed = true;
					view.info.ground_sep = view.min + rel_ground;
					view.info.crown_sep = view.info.crown_sep.max(view.info.ground_sep);
				}
				let mut rel_crown = view.info.crown_sep - view.min;
				if ui
					.add(egui::Slider::new(&mut rel_crown, 0.0..=view.height).suffix("m"))
					.changed()
				{
					changed = true;
					view.info.crown_sep = view.min + rel_crown;
					view.info.ground_sep = view.info.ground_sep.min(view.info.crown_sep);
				}
				if changed {
					view.property = SegmentView::gen_property(
						&self.state,
						self.segments.get(&view.index).unwrap(),
						view.info.ground_sep,
						view.info.crown_sep,
					);
				}
				ui.label("trunk diameter");
				ui.label(format!("{}m", view.info.trunk_diameter));
				ui.label("crown diameter");
				ui.label(format!("{}m", view.info.crown_diameter));

				if ui.button("Export Options...").clicked() {
					println!("todo");
				}
			},
		};

		response
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
			Modus::Spawn => {
				let Some((_, distance)) = self.select(start, direction) else {
					return;
				};
				let hit = start + direction * distance;
				let mut target = Segment::empty();
				let mut empty: Vec<usize> = Vec::new();
				for (&other, segment) in self.segments.iter_mut() {
					let seg_changed = segment.remove(hit, self.draw_radius, Some(&mut target));
					if segment.points.is_empty() {
						empty.push(other);
					} else if seg_changed {
						self.point_clouds
							.insert(other, segment.render_data(&self.state, other));
					}
				}
				for empty in empty {
					self.segments.remove(&empty);
					self.point_clouds.remove(&empty);
				}
				if target.points.is_empty() {
					return;
				}

				let mut idx = rand::random();
				while self.segments.contains_key(&idx) {
					idx = rand::random();
				}
				self.point_clouds
					.insert(idx, target.render_data(&self.state, idx));
				self.segments.insert(idx, target);
				self.modus = Modus::Draw(idx);
			},
			Modus::Delete => {},

			Modus::SelectView => {
				let Some((idx, _)) = self.select(start, direction) else {
					return;
				};
				self.modus = Modus::View(SegmentView::new(
					&self.state,
					idx,
					self.segments.get(&idx).unwrap(),
				));
			},

			Modus::View(view) => {
				let seg = self.segments.get_mut(&view.index).unwrap();
				let Some(distance) = seg.exact_distance(start, direction) else {
					return;
				};

				let hit = start + direction * distance;
				if seg.remove(hit, self.draw_radius, None) {
					self.point_clouds
						.insert(view.index, seg.render_data(&self.state, view.index));
				}
			},
		}
	}

	pub fn drag(&mut self, start: na::Point3<f32>, direction: na::Vector3<f32>) {
		match &self.modus {
			Modus::Delete => {
				let Some((_, distance)) = self.select(start, direction) else {
					return;
				};
				let hit = start + direction * distance;
				let mut empty = Vec::new();
				for (&other, segment) in self.segments.iter_mut() {
					if segment.remove(hit, self.draw_radius, None) {
						self.point_clouds
							.insert(other, segment.render_data(&self.state, other));
					}
					if segment.points.is_empty() {
						empty.push(other);
					}
				}
				for empty in empty {
					self.segments.remove(&empty);
					self.point_clouds.remove(&empty);
				}
			},
			&Modus::Draw(idx) => {
				let Some((_, distance)) = self.select(start, direction) else {
					return;
				};
				let hit = start + direction * distance;
				let mut target = self.segments.remove(&idx).unwrap();
				let mut changed = false;
				let mut empty = Vec::new();
				for (&other, segment) in self.segments.iter_mut() {
					if segment.remove(hit, self.draw_radius, Some(&mut target)) {
						self.point_clouds
							.insert(other, segment.render_data(&self.state, other));
						changed = true;
					}
					if segment.points.is_empty() {
						empty.push(other);
					}
				}
				if changed {
					self.point_clouds
						.insert(idx, target.render_data(&self.state, idx));
				}
				self.segments.insert(idx, target);
				for empty in empty {
					self.segments.remove(&empty);
					self.point_clouds.remove(&empty);
				}
			},
			Modus::View(_view) => {
				// delete in view
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
	Spawn,
	Delete,
	View(SegmentView),
}

pub enum InteractiveResponse {
	None,
}

#[derive(Debug)]
pub struct SegmentView {
	pub index: usize,
	min: f32,
	height: f32,
	info: SegmentInformation,
	pub property: render::PointCloudProperty,
}

impl SegmentView {
	fn new(state: &render::State, index: usize, segment: &Segment) -> Self {
		let min = segment.min.y;
		let max = segment.max.y;
		let info = SegmentInformation::new(&segment.points, min, max);
		Self {
			index,
			min,
			info,
			height: max - min,
			property: Self::gen_property(state, segment, info.ground_sep, info.crown_sep),
		}
	}

	fn gen_property(
		state: &render::State,
		segment: &Segment,
		ground_sep: f32,
		crown_sep: f32,
	) -> render::PointCloudProperty {
		let mut property = vec![0u32; segment.points.len()];
		for (idx, p) in segment.points.iter().enumerate() {
			property[idx] = if p.y < ground_sep {
				0
			} else if p.y < crown_sep {
				u32::MAX / 2
			} else {
				u32::MAX
			};
		}
		render::PointCloudProperty::new(state, &property)
	}
}
