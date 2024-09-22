use std::{
	collections::{HashMap, HashSet},
	hash::Hash,
	ops::Not,
};

use crate::{
	calculations::{Classification, SegmentData},
	environment::{self, Saver},
	id,
};
use nalgebra as na;

/// Filter points based on Classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeMode {
	All,
	Crown,
	Trunk,
	Ground,
	Tree,
	NoCrown,
}

impl IncludeMode {
	pub fn valid(self, c: Classification) -> bool {
		match self {
			Self::All => true,
			Self::Crown => c == Classification::Crown,
			Self::Trunk => c == Classification::Trunk,
			Self::Ground => c == Classification::Ground,
			Self::Tree => matches!(c, Classification::Crown | Classification::Trunk),
			Self::NoCrown => matches!(c, Classification::Trunk | Classification::Ground),
		}
	}

	pub fn name(self) -> &'static str {
		match self {
			Self::All => "All",
			Self::Crown => "Crown",
			Self::Trunk => "Trunk",
			Self::Ground => "Ground",
			Self::Tree => "Tree",
			Self::NoCrown => "No Crown",
		}
	}

	pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
		let mut changed = false;
		egui::ComboBox::from_id_salt(id!())
			.selected_text(self.name())
			.width(ui.available_width())
			.show_ui(ui, |ui| {
				for v in [
					Self::All,
					Self::Crown,
					Self::Trunk,
					Self::Ground,
					Self::Tree,
					Self::NoCrown,
				] {
					changed |= ui.selectable_value(self, v, v.name()).changed();
				}
			});
		changed
	}
}

/// Hull for the selected segment.
#[derive(Debug)]
pub enum Hull {
	None,
	Convex(ConvexHull),
	RadialBoundingVolume(RadialBoundingVolume),
	SplitRadialBoundingVolume(SplitRadialBoundingVolume),
}

impl Hull {
	pub fn update(
		&mut self,
		segment: &SegmentData,
		transform: Option<na::Affine3<f32>>,
		state: &render::State,
	) {
		match self {
			Self::None => {},
			Self::Convex(convex) => {
				*convex = ConvexHull::new(
					&segment.points,
					&segment.classifications,
					convex.mode,
					state,
				)
			},
			Self::RadialBoundingVolume(rbv) => {
				rbv.update(segment, transform, state);
			},
			Self::SplitRadialBoundingVolume(split) => {
				split.crown.update(segment, transform, state);
				split.trunk.update(segment, transform, state);
			},
		}
	}

	pub fn render<'a>(
		&'a self,
		cloud: &'a render::PointCloud,
		lines_pass: &mut render::LinesPass<'a>,
	) {
		match self {
			Self::None => {},
			Self::Convex(convex) => {
				convex.lines.render(cloud, lines_pass);
			},
			Self::RadialBoundingVolume(rbv) => {
				rbv.visual_lines.render(&rbv.visual_points, lines_pass);
			},
			Self::SplitRadialBoundingVolume(split) => {
				split
					.crown
					.visual_lines
					.render(&split.crown.visual_points, lines_pass);
				split
					.trunk
					.visual_lines
					.render(&split.trunk.visual_points, lines_pass);
			},
		}
	}

	pub fn ui(
		&mut self,
		ui: &mut egui::Ui,
		segment: &SegmentData,
		transform: Option<na::Affine3<f32>>,
		state: &render::State,
	) {
		ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Hull"));
		if ui
			.add(egui::RadioButton::new(matches!(self, Hull::None), "None"))
			.clicked()
		{
			*self = Hull::None;
		}
		if ui
			.add(egui::RadioButton::new(
				matches!(self, Hull::Convex(_)),
				"Convex Hull",
			))
			.clicked()
		{
			*self = Hull::Convex(ConvexHull::new(
				&segment.points,
				&segment.classifications,
				IncludeMode::Crown,
				state,
			));
		}
		if ui
			.add(egui::RadioButton::new(
				matches!(self, Hull::RadialBoundingVolume(_)),
				"Radial Bounding Volume",
			))
			.clicked()
		{
			*self = Hull::RadialBoundingVolume(RadialBoundingVolume::new(
				IncludeMode::All,
				RadialBoundingVolumeMethod::Max,
				false,
				&segment.points,
				&segment.classifications,
				8,
				8,
				state,
				transform,
			));
		}
		if ui
			.add(egui::RadioButton::new(
				matches!(self, Hull::SplitRadialBoundingVolume(_)),
				"Split Radial Bounding Volume",
			))
			.clicked()
		{
			*self = Hull::SplitRadialBoundingVolume(SplitRadialBoundingVolume {
				crown: RadialBoundingVolume::new(
					IncludeMode::Crown,
					RadialBoundingVolumeMethod::Max,
					false,
					&segment.points,
					&segment.classifications,
					26,
					32,
					state,
					transform,
				),
				trunk: RadialBoundingVolume::new(
					IncludeMode::Trunk,
					RadialBoundingVolumeMethod::Max,
					false,
					&segment.points,
					&segment.classifications,
					5,
					32,
					state,
					transform,
				),
			});
		}
		match self {
			Self::None => {},
			Self::Convex(convex) => {
				ui.separator();
				ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Convex Hull"));
				let mut changed = false;
				egui::Grid::new(id!()).num_columns(2).show(ui, |ui| {
					ui.label("Include");
					changed |= convex.mode.ui(ui);
					ui.end_row();
				});
				if changed {
					*convex = ConvexHull::new(
						&segment.points,
						&segment.classifications,
						convex.mode,
						state,
					);
				}
				if ui
					.add_sized([ui.available_width(), 0.0], egui::Button::new("Save"))
					.clicked()
				{
					let points = segment.points.clone();
					let faces = convex.faces.clone();
					environment::Saver::start("convex_hull", "ply", move |mut saver| {
						ConvexHull::save(&mut saver, &points, &faces).unwrap();
						saver.save();
					})
				}
			},
			Self::RadialBoundingVolume(rbv) => {
				ui.separator();
				ui.add_sized(
					[ui.available_width(), 0.0],
					egui::Label::new("Radial Bounding Volume"),
				);
				let mut changed = false;
				egui::Grid::new(id!()).num_columns(2).show(ui, |ui| {
					ui.label("Include");
					changed |= rbv.mode.ui(ui);
					ui.end_row();

					ui.label("Method");
					egui::ComboBox::from_id_salt(id!())
						.selected_text(format!("{:?}", rbv.method))
						.width(ui.available_width())
						.show_ui(ui, |ui| {
							for v in [
								RadialBoundingVolumeMethod::Max,
								RadialBoundingVolumeMethod::Mean,
							] {
								changed |= ui
									.selectable_value(&mut rbv.method, v, format!("{:?}", v))
									.changed();
							}
						});
					ui.end_row();

					ui.label("Slices");
					changed |= ui.add(egui::Slider::new(&mut rbv.slices, 1..=32)).changed();
					ui.end_row();

					ui.label("Sectors");
					changed |= ui
						.add(egui::Slider::new(&mut rbv.sectors, 3..=32))
						.changed();
					ui.end_row();

					let enabled = rbv.sectors % 2 == 0;
					ui.add_enabled_ui(enabled, |ui| ui.label("Extra"));
					ui.add_enabled_ui(enabled, |ui| {
						changed |= ui.checkbox(&mut rbv.symmetric, "Symmetric").changed()
					});
					ui.end_row();
				});
				if ui
					.add_sized(
						[ui.available_width(), 0.0],
						egui::Button::new("Save Distances"),
					)
					.clicked()
				{
					let save = RadialBoundingVolumeDistances {
						center_x: rbv.center.x,
						center_y: rbv.center.y,
						height_min: rbv.min,
						distances: rbv.distances.clone(),
						slices: rbv.slices,
						sectors: rbv.sectors,
					};

					environment::Saver::start("radial_bounding_volume", "json", move |mut saver| {
						serde_json::to_writer_pretty(saver.inner(), &save).unwrap();
						saver.save();
					})
				}

				if ui
					.add_sized(
						[ui.available_width(), 0.0],
						egui::Button::new("Save Landmarks"),
					)
					.clicked()
				{
					let landmarks = rbv.landmarks(0.0);

					environment::Saver::start("landmarks", "txt", move |mut saver| {
						use std::io::Write;

						let mut writer = saver.inner();
						for (idx, value) in landmarks.iter().copied().enumerate() {
							if idx == landmarks.len() - 1 {
								write!(writer, "{}\n", value).unwrap();
							} else {
								write!(writer, "{}\t", value).unwrap();
							}
						}
						drop(writer);
						saver.save();
					});
				}

				if changed {
					rbv.update(segment, transform, state);
				}
			},
			Self::SplitRadialBoundingVolume(split) => {
				ui.separator();
				ui.add_sized(
					[ui.available_width(), 0.0],
					egui::Label::new("Split Radial Bounding Volume"),
				);
				let mut changed = false;
				egui::Grid::new(id!()).num_columns(2).show(ui, |ui| {
					ui.label("Crown Slices");
					changed |= ui
						.add(egui::Slider::new(&mut split.crown.slices, 1..=32))
						.changed();
					ui.end_row();

					ui.label("Crown Sectors");
					changed |= ui
						.add(egui::Slider::new(&mut split.crown.sectors, 3..=32))
						.changed();
					ui.end_row();

					let enabled = split.crown.sectors % 2 == 0;
					ui.add_enabled_ui(enabled, |ui| ui.label("Crown"));
					ui.add_enabled_ui(enabled, |ui| {
						changed |= ui
							.checkbox(&mut split.crown.symmetric, "Symmetric")
							.changed()
					});
					ui.end_row();

					ui.label("Trunk Slices");
					changed |= ui
						.add(egui::Slider::new(&mut split.trunk.slices, 1..=32))
						.changed();
					ui.end_row();

					ui.label("Trunk Sectors");
					changed |= ui
						.add(egui::Slider::new(&mut split.trunk.sectors, 3..=32))
						.changed();
					ui.end_row();

					let enabled = split.trunk.sectors % 2 == 0;
					ui.add_enabled_ui(enabled, |ui| ui.label("Trunk"));
					ui.add_enabled_ui(enabled, |ui| {
						changed |= ui
							.checkbox(&mut split.trunk.symmetric, "Symmetric")
							.changed()
					});
					ui.end_row();
				});
				if changed {
					split.crown.update(segment, transform, state);
					split.trunk.update(segment, transform, state);
				}
				if ui
					.add_sized(
						[ui.available_width(), 0.0],
						egui::Button::new("Save Landmarks"),
					)
					.clicked()
				{
					let mut landmarks = split.trunk.landmarks(0.0);
					let base = split.crown.min - split.trunk.min;
					landmarks.extend_from_slice(&split.crown.landmarks(base));
					let top = base + split.crown.slice_height * split.crown.slices as f32;
					landmarks.extend_from_slice(&[0.0, 0.0, top]);

					environment::Saver::start("landmarks", "txt", move |mut saver| {
						use std::io::Write;

						let mut writer = saver.inner();
						for (idx, value) in landmarks.iter().copied().enumerate() {
							if idx == landmarks.len() - 1 {
								write!(writer, "{}\n", value).unwrap();
							} else {
								write!(writer, "{}\t", value).unwrap();
							}
						}
						drop(writer);
						saver.save();
					});
				}
				ui.end_row();

				if ui
					.add_sized(
						[ui.available_width(), 0.0],
						egui::Button::new("Save Traits"),
					)
					.clicked()
				{
					let save = split.traits();
					environment::Saver::start("traits", "json", move |mut saver| {
						serde_json::to_writer_pretty(saver.inner(), &save).unwrap();
						saver.save();
					})
				}
			},
		}
	}
}

/// Data for the convex hull.
#[derive(Debug)]
pub struct ConvexHull {
	mode: IncludeMode,
	faces: Vec<[u32; 3]>,
	lines: render::Lines,
}

impl ConvexHull {
	/// Calculate convex hull with the gift wrapping algorithm.
	///
	/// Source: https://tildesites.bowdoin.edu/~ltoma/teaching/cs3250-CompGeom/spring17/Lectures/cg-hull3d.pdf
	pub fn new(
		points: &[na::Point3<f32>],
		classifications: &[Classification],
		mode: IncludeMode,
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
			.filter_map(|((idx, pos), &c)| mode.valid(c).then_some(Point { idx, pos }))
			.collect::<Vec<_>>();

		if points.len() < 10 {
			return Self {
				faces: Vec::new(),
				lines: render::Lines::new(state, &[0, 0]),
				mode,
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
			mode,
		}
	}

	/// Save the convex hull as `.ply`.
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

/// Approximate hull with cylinder seperated into slices and sectors.
///
/// Source: https://scholar.google.com/scholar?hl=en&as_sdt=0%2C5&q=Learning+to+Reconstruct+Botanical+Trees+from+Single+Images&btnG=
#[derive(Debug)]
pub struct RadialBoundingVolume {
	mode: IncludeMode,
	method: RadialBoundingVolumeMethod,
	symmetric: bool,

	center: na::Point2<f32>,
	min: f32,
	distances: Vec<f32>,
	slices: usize,
	sectors: usize,
	slice_height: f32,

	visual_points: render::PointCloud,
	visual_lines: render::Lines,
}

/// Method used to calculate the distance to the center
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadialBoundingVolumeMethod {
	Max,
	Mean,
	// Percentile?,
	// TopN?,
}

impl RadialBoundingVolume {
	pub fn new(
		mode: IncludeMode,
		method: RadialBoundingVolumeMethod,
		symmetric: bool,
		points: &[na::Point3<f32>],
		classifications: &[Classification],
		slices: usize,
		sectors: usize,
		state: &render::State,

		transform: Option<na::Affine3<f32>>,
	) -> Self {
		let (transform, centered) = match transform {
			Some(transform) => (transform, true),
			None => (na::Affine3::identity(), false),
		};
		let inv = transform.inverse();

		let points_iter = points
			.iter()
			.zip(classifications)
			.filter_map(|(&p, &c)| mode.valid(c).then_some(p))
			.map(|p| inv * p);

		let mut points = points_iter.clone();
		let Some(first) = points.next() else {
			return Self {
				mode,
				method,

				symmetric,

				center: na::point![0.0, 0.0],
				min: 0.0,
				distances: Vec::new(),
				slice_height: 1.0,
				slices,
				sectors,
				visual_points: render::PointCloud::new(state, &[na::point![0.0, 0.0, 0.0]]),
				visual_lines: render::Lines::new(state, &[0]),
			};
		};

		let mut min = first.y;
		let mut max = min;
		for p in points.clone() {
			min = min.min(p.y);
			max = max.max(p.y);
		}

		let center = if centered.not() {
			let mut center = na::point![first.x, first.z];
			// calculate center, radius min and max
			// this is an approximation
			// source: <https://en.wikipedia.org/wiki/Bounding_sphere#Ritter%27s_bounding_sphere>
			let mut radius = 0.0;

			for p in points {
				let p = na::point![p.x, p.z];
				let dist = (p - center).norm();
				if dist <= radius {
					continue;
				}
				radius = (radius + dist) / 2.0;
				center += (dist - radius) * (p - center) / dist;
			}
			center
		} else {
			na::point![0.0, 0.0]
		};

		// calculate distances
		let slice_height = (max - min) / slices as f32;
		let sector_angle = std::f32::consts::TAU / sectors as f32;

		let get_idx_and_distance = |p: na::Point3<f32>| {
			let slice = ((p.y - min) / slice_height).floor() as usize;
			let slice = slice.min(slices - 1);

			let delta = na::point![p.x, p.z] - center;
			let distance = delta.norm();

			let angle = f32::atan2(delta.y, delta.x) + std::f32::consts::TAU;
			let sector = ((angle / sector_angle) % sectors as f32).floor() as usize;

			(slice * sectors + sector, distance)
		};

		let mut distances = vec![0.0f32; slices * sectors];
		match method {
			RadialBoundingVolumeMethod::Max => {
				for p in points_iter.clone() {
					let (idx, distance) = get_idx_and_distance(p);
					distances[idx] = distances[idx].max(distance);
				}
			},
			RadialBoundingVolumeMethod::Mean => {
				let mut counts = vec![0; distances.len()];
				for p in points_iter.clone() {
					let (idx, distance) = get_idx_and_distance(p);
					distances[idx] += distance;
					counts[idx] += 1;
				}
				for (distance, count) in distances.iter_mut().zip(counts) {
					*distance /= count as f32;
				}
			},
		}

		// maybe make symmetric
		if symmetric && sectors % 2 == 0 {
			for slice in 0..slices {
				for sector in 0..(sectors / 2) {
					let idx_0 = slice * sectors + sector;
					let idx_1 = slice * sectors + sector + sectors / 2;
					let val = (distances[idx_0] + distances[idx_1]) / 2.0;
					distances[idx_0] = val;
					distances[idx_1] = val;
				}
			}
		}

		// create render data
		let mut points = Vec::new();
		let mut indices = Vec::new();
		let mut line = |a, b| {
			indices.push(points.len() as u32);
			points.push(transform * a);
			indices.push(points.len() as u32);
			points.push(transform * b);
		};

		for slice in 0..slices {
			for sector in 0..sectors {
				let distance = distances[slice * sectors + sector];

				let y_min = min + slice_height * slice as f32;
				let y_max = y_min + slice_height;

				let mut angle = sector_angle * sector as f32;

				let mut x = center.x + angle.cos() * distance;
				let mut z = center.y + angle.sin() * distance;

				line(
					na::point![center.x, y_min, center.y],
					na::point![center.x, y_max, center.y],
				);

				line(
					na::point![center.x, y_min, center.y],
					na::point![x, y_min, z],
				);
				line(
					na::point![center.x, y_max, center.y],
					na::point![x, y_max, z],
				);
				line(na::point![x, y_min, z], na::point![x, y_max, z]);

				let details = (sector_angle * distance / 0.5).ceil() as usize;
				for _ in 0..details {
					angle = (angle + sector_angle / details as f32) % std::f32::consts::TAU;
					let next_x = center.x + angle.cos() * distance;
					let next_z = center.y + angle.sin() * distance;

					line(na::point![x, y_min, z], na::point![next_x, y_min, next_z]);
					line(na::point![x, y_max, z], na::point![next_x, y_max, next_z]);

					x = next_x;
					z = next_z;
				}
				line(na::point![x, y_min, z], na::point![x, y_max, z]);
				line(
					na::point![center.x, y_min, center.y],
					na::point![x, y_min, z],
				);
				line(
					na::point![center.x, y_max, center.y],
					na::point![x, y_max, z],
				);
			}
		}

		let visual_points = render::PointCloud::new(state, &points);
		let visual_lines = render::Lines::new(state, &indices);

		Self {
			mode,
			method,
			symmetric,

			distances,
			slice_height,
			center,
			slices,
			min,

			sectors,
			visual_points,
			visual_lines,
		}
	}

	pub fn update(
		&mut self,
		segment: &SegmentData,
		transform: Option<na::Affine3<f32>>,
		state: &render::State,
	) {
		*self = RadialBoundingVolume::new(
			self.mode,
			self.method,
			self.symmetric,
			&segment.points,
			&segment.classifications,
			self.slices,
			self.sectors,
			state,
			transform,
		)
	}

	/// Calculate characteristic points.
	fn landmarks(&self, base: f32) -> Vec<f32> {
		let mut values = Vec::with_capacity(self.slices * self.sectors * 3);

		let sector_angle = std::f32::consts::TAU / self.sectors as f32;
		for slice in 0..self.slices {
			for sector in 0..self.sectors {
				let idx = slice * self.sectors + sector;
				let distance = self.distances[idx];
				let angle = (sector as f32 + 0.5) * sector_angle;
				values.push(angle.cos() * distance);
				values.push(angle.sin() * distance);
				let offset = match self.mode {
					// interpolate from 0.0 at lowest to 1.0 at highest
					IncludeMode::Ground | IncludeMode::NoCrown => {
						slice as f32 / (self.slices - 1) as f32
					},
					// middle of the slice
					_ => 0.5,
				};

				values.push(base + (slice as f32 + offset) * self.slice_height);
			}
		}
		values
	}
}

/// Information to save Radial Bounding Volume.
#[derive(Debug, serde::Serialize)]
pub struct RadialBoundingVolumeDistances {
	center_x: f32,
	center_y: f32,
	height_min: f32,
	slices: usize,
	sectors: usize,
	distances: Vec<f32>,
}

/// Seperate radial bounding volumes for the crown and trunk.
#[derive(Debug)]
pub struct SplitRadialBoundingVolume {
	crown: RadialBoundingVolume,
	trunk: RadialBoundingVolume,
}

impl SplitRadialBoundingVolume {
	pub fn traits(&self) -> Traits {
		let trunk_height = self.trunk.slice_height * self.trunk.slices as f32;
		let height =
			self.crown.min + self.crown.slice_height * self.crown.slices as f32 - self.trunk.min;

		let diameter_breast_height = {
			let slice = (1.3 / self.trunk.slice_height).floor() as usize;
			let slice = slice.min(self.trunk.slices - 1);
			let range = (slice * self.trunk.sectors)..((slice + 1) * self.trunk.sectors);
			self.trunk.distances[range].iter().sum::<f32>() * 2.0 / self.trunk.sectors as f32
		};

		let trunk_cross_area = std::f32::consts::PI * (diameter_breast_height / 2.0).powi(2);

		let mut sector_max = vec![0.0f32; self.crown.sectors];
		for slice in 0..self.crown.slices {
			for sector in 0..self.crown.sectors {
				let idx = slice * self.crown.sectors + sector;
				sector_max[sector] = sector_max[sector].max(self.crown.distances[idx]);
			}
		}

		let mut crown_sectors_sum = 0.0;
		for slice in 0..self.crown.slices {
			for sector in 0..self.crown.sectors {
				let idx = slice * self.crown.sectors + sector;
				crown_sectors_sum += self.crown.distances[idx];
			}
		}

		let crown_diameter =
			sector_max.iter().copied().sum::<f32>() * 2.0 / self.crown.sectors as f32;

		let crown_projected_area = sector_max.iter().copied().map(|x| x.powi(2)).sum::<f32>()
			* std::f32::consts::PI
			/ self.crown.sectors as f32;

		let crown_volume = {
			let mut crown_volume = 0.0;
			for slice in 0..self.crown.slices {
				for sector in 0..self.crown.sectors {
					let idx = slice * self.crown.sectors + sector;
					crown_volume += self.crown.distances[idx].powi(2);
				}
			}
			crown_volume * std::f32::consts::PI * self.crown.slice_height
				/ self.crown.sectors as f32
		};

		let crown_surface = 2.0 * crown_projected_area
			+ std::f32::consts::TAU * self.crown.slice_height / self.crown.sectors as f32
				* crown_sectors_sum;

		let stem_volume = trunk_cross_area * (trunk_height + (height - trunk_height) / 3.0);
		let wood_volume = stem_volume
			+ trunk_cross_area / (self.crown.slices as f32 * self.crown.sectors as f32)
				* crown_sectors_sum;

		Traits {
			diameter_breast_height,
			trunk_cross_area,
			crown_diameter,
			crown_projected_area,
			crown_volume,
			crown_surface,
			stem_volume,
			wood_volume,
		}
	}
}

/// Characterisic traits for the tree.
#[derive(Debug, serde::Serialize)]
pub struct Traits {
	diameter_breast_height: f32,
	trunk_cross_area: f32,
	crown_diameter: f32,
	crown_projected_area: f32,
	crown_volume: f32,
	crown_surface: f32,
	stem_volume: f32,
	wood_volume: f32,
}
