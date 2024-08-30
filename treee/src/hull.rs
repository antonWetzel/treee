use std::{
	collections::{HashMap, HashSet},
	hash::Hash,
	ops::Not,
};

use crate::{
	calculations::{Classification, SegmentData},
	environment::{self, Saver},
	id,
	program::DisplaySettings,
};
use nalgebra as na;

/// Filter points based on Classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeMode {
	Crown,
	All,
}

impl IncludeMode {
	pub fn valid(self, c: Classification) -> bool {
		match self {
			Self::All => true,
			Self::Crown => c == Classification::Crown,
		}
	}
}

/// Hull for the selected segment.
#[derive(Debug)]
pub enum Hull {
	None,
	Convex(ConvexHull),
	RadialBoundingVolume(RadialBoundingVolume),
}

impl Hull {
	pub fn update(&mut self, segment: &SegmentData, state: &render::State) {
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
				*rbv = RadialBoundingVolume::new(
					rbv.mode,
					&segment.points,
					&segment.classifications,
					rbv.slices,
					rbv.sectors,
					state,
				)
			},
		}
	}

	pub fn render<'a>(
		&'a self,
		cloud: &'a render::PointCloud,
		render_pass: &mut render::RenderPass<'a>,
		lines_state: &'a render::LinesState,
		display_settings: &'a DisplaySettings,
	) {
		match self {
			Self::None => {},
			Self::Convex(ref convex) => {
				let lines_pass = lines_state.render(render_pass, display_settings.camera.gpu());
				convex.lines.render(cloud, lines_pass);
			},
			Self::RadialBoundingVolume(ref rbv) => {
				let lines_pass = lines_state.render(render_pass, display_settings.camera.gpu());
				rbv.visual_lines.render(&rbv.visual_points, lines_pass);
			},
		}
	}

	pub fn ui(&mut self, ui: &mut egui::Ui, segment: &SegmentData, state: &render::State) {
		match self {
			Self::None => {},
			Self::Convex(convex) => {
				ui.separator();
				ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Convex Hull"));
				let mut changed = false;
				egui::Grid::new(id!()).num_columns(2).show(ui, |ui| {
					ui.label("Include");
					ui.horizontal(|ui| {
						changed |= ui
							.radio_value(&mut convex.mode, IncludeMode::Crown, "Crown")
							.changed();
						changed |= ui
							.radio_value(&mut convex.mode, IncludeMode::All, "All")
							.changed();
					});
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
					environment::Saver::start("convex_hull.ply", move |mut saver| {
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
					ui.horizontal(|ui| {
						changed |= ui
							.radio_value(&mut rbv.mode, IncludeMode::Crown, "Crown")
							.changed();
						changed |= ui
							.radio_value(&mut rbv.mode, IncludeMode::All, "All")
							.changed();
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
				});
				if ui
					.add_sized([ui.available_width(), 0.0], egui::Button::new("Save"))
					.clicked()
				{
					let save = RadialBoundingVolumeSave {
						center_x: rbv.center.x,
						center_y: rbv.center.y,
						height_min: rbv.min,
						distances: rbv.distances.clone(),
						slices: rbv.slices,
						sectors: rbv.sectors,
					};

					environment::Saver::start("radial_bounding_volume.json", move |mut saver| {
						serde_json::to_writer_pretty(saver.inner(), &save).unwrap();
						saver.save();
					})
				}

				if changed {
					*rbv = RadialBoundingVolume::new(
						rbv.mode,
						&segment.points,
						&segment.classifications,
						rbv.slices,
						rbv.sectors,
						state,
					)
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

		let valid = match mode {
			IncludeMode::Crown => |c| c == Classification::Crown,
			IncludeMode::All => |_| true,
		};

		let points = points
			.iter()
			.copied()
			.enumerate()
			.zip(classifications)
			.filter_map(|((idx, pos), &c)| valid(c).then_some(Point { idx, pos }))
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

	center: na::Point2<f32>,
	min: f32,
	distances: Vec<f32>,
	slices: usize,
	sectors: usize,

	visual_points: render::PointCloud,
	visual_lines: render::Lines,
}

impl RadialBoundingVolume {
	pub fn new(
		mode: IncludeMode,
		points: &[na::Point3<f32>],
		classifications: &[Classification],
		slices: usize,
		sectors: usize,
		state: &render::State,
	) -> Self {
		let points_iter = points
			.iter()
			.zip(classifications)
			.filter_map(|(&p, &c)| mode.valid(c).then_some(p));

		let mut points = points_iter.clone();
		let Some(first) = points.next() else {
			return Self {
				mode,
				center: na::point![0.0, 0.0],
				min: 0.0,
				distances: Vec::new(),
				slices,
				sectors,
				visual_points: render::PointCloud::new(state, &[na::point![0.0, 0.0, 0.0]]),
				visual_lines: render::Lines::new(state, &[0]),
			};
		};

		// calculate center, radius min and max
		// this is an approximation
		// source: <https://en.wikipedia.org/wiki/Bounding_sphere#Ritter%27s_bounding_sphere>
		let mut radius = 0.0;
		let mut center = na::point![first.x, first.z];
		let mut min = first.y;
		let mut max = min;
		for p in points {
			min = min.min(p.y);
			max = max.max(p.y);

			let p = na::point![p.x, p.z];
			let dist = (p - center).norm();
			if dist <= radius {
				continue;
			}
			radius = (radius + dist) / 2.0;
			center += (dist - radius) * (p - center) / dist;
		}

		// calculate distances
		let slice_height = (max - min) / slices as f32;
		let sector_angle = std::f32::consts::TAU / sectors as f32;
		let mut distances = vec![0.0f32; slices * sectors];
		for p in points_iter.clone() {
			let slice = ((p.y - min) / slice_height).floor() as usize;
			let slice = slice.min(slices - 1);

			let delta = na::point![p.x, p.z] - center;
			let distance = delta.norm();

			let angle = f32::atan2(delta.y, delta.x) + std::f32::consts::TAU;
			let sector = ((angle / sector_angle) % sectors as f32).floor() as usize;

			let idx = slice * sectors + sector;
			distances[idx] = distances[idx].max(distance);
		}

		// create render data
		let mut points = Vec::new();
		let mut indices = Vec::new();
		let mut line = |a, b| {
			indices.push(points.len() as u32);
			points.push(a);
			indices.push(points.len() as u32);
			points.push(b);
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
			distances,

			center,
			slices,
			min,

			sectors,
			visual_points,
			visual_lines,
		}
	}
}

/// Information to save Radial Bounding Volume.
#[derive(Debug, serde::Serialize)]
pub struct RadialBoundingVolumeSave {
	center_x: f32,
	center_y: f32,
	height_min: f32,
	slices: usize,
	sectors: usize,
	distances: Vec<f32>,
}
