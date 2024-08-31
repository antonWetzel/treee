use nalgebra as na;

use crate::{
	calculations::{Classification, SegmentData},
	id,
};

/// Algorithm to calculate the trunk origin and axis
#[derive(Debug, Clone, Copy)]
pub enum TrunkAxisAlgorithm {
	None,
	AverageDirection(usize),
	LowHighLayer(f32),
}

/// Data to render the trunk axis
#[derive(Debug)]
pub struct TrunkAxisRender {
	cloud: render::PointCloud,
	lines: render::Lines,
}

impl TrunkAxisRender {
	pub fn new(
		origin: na::Point3<f32>,
		direction: na::Vector3<f32>,
		scale: f32,
		state: &render::State,
	) -> Self {
		let vertices = [
			origin,
			origin + direction * scale,
			origin - na::Vector3::x() * 2.5,
			origin + na::Vector3::x() * 2.5,
			origin - na::Vector3::z() * 2.5,
			origin + na::Vector3::z() * 2.5,
		];
		let indices = [0, 1, 2, 3, 4, 5];
		Self {
			cloud: render::PointCloud::new(state, &vertices),
			lines: render::Lines::new(state, &indices),
		}
	}
}

/// Origin and axis for the trunk
#[derive(Debug)]
pub struct TrunkAxis {
	origin: na::Point3<f32>,
	direction: na::Vector3<f32>,
	algorithm: TrunkAxisAlgorithm,
	render: Option<TrunkAxisRender>,
}

impl TrunkAxis {
	pub fn transform(&self) -> Option<na::Affine3<f32>> {
		if matches!(self.algorithm, TrunkAxisAlgorithm::None) {
			return None;
		}

		let y = self.direction;
		let x = y.cross(&na::Vector3::z());
		let z = x.cross(&y);
		let rot = na::Rotation3::from_matrix_unchecked(na::Matrix::from_columns(&[x, y, z]));

		let transform = na::Affine3::identity()
			* na::Translation3::new(self.origin.x, self.origin.y, self.origin.z)
			* rot;
		Some(transform)
	}

	pub fn update(&mut self, segment: &SegmentData, state: &render::State) {
		*self = Self::new(
			&segment.points,
			&segment.classifications,
			self.algorithm,
			state,
		);
	}

	pub fn new(
		points: &[na::Point3<f32>],
		classifications: &[Classification],
		algorithm: TrunkAxisAlgorithm,
		state: &render::State,
	) -> Self {
		match algorithm {
			TrunkAxisAlgorithm::None => Self::new_empty(algorithm),
			TrunkAxisAlgorithm::AverageDirection(layers) => {
				Self::new_average_direction(points, classifications, layers, state)
			},
			TrunkAxisAlgorithm::LowHighLayer(layer) => {
				Self::new_low_high_layer(points, classifications, layer, state)
			},
		}
	}

	pub fn new_empty(algorithm: TrunkAxisAlgorithm) -> Self {
		Self {
			origin: na::point![0.0, 0.0, 0.0],
			direction: na::Vector3::y(),
			algorithm,
			render: None,
		}
	}

	/// Split trunk into layers and calculate the average direction from the origin to layer means
	pub fn new_average_direction(
		points: &[na::Point3<f32>],
		classifications: &[Classification],
		layers: usize,
		state: &render::State,
	) -> Self {
		let points = points
			.iter()
			.zip(classifications)
			.filter_map(|(&p, &c)| (c == Classification::Trunk).then_some(p))
			.collect::<Vec<_>>();

		if points.is_empty() {
			log::warn!("No points for trunk");
			return Self::new_empty(TrunkAxisAlgorithm::AverageDirection(layers));
		}
		let (mut min, mut max) = (points[0].y, points[0].y);
		for p in points[1..].iter().copied() {
			min = min.min(p.y);
			max = max.max(p.y);
		}

		let mut sum = na::vector![0.0, 0.0];
		let mut count = 0;

		let range = min + 0.4..min + 0.6;
		for p in points.iter().copied() {
			if range.contains(&p.y) {
				sum += na::vector![p.x, p.z];
				count += 1;
			}
		}

		let layer_height = (max - min) / layers as f32;
		let mut means = vec![(na::vector![0.0, 0.0], 0); layers];
		for p in points.iter().copied() {
			let idx = ((p.y - min) / layer_height).floor() as usize;
			let idx = idx.min(layers - 1);
			means[idx].0 += na::vector![p.x, p.z];
			means[idx].1 += 1;
		}

		if count == 0 {
			// use lowest layer as fallback
			sum = means[0].0 / means[0].1 as f32;
		} else {
			sum /= count as f32;
		}

		let origin = na::point![sum.x, min, sum.y];

		let mut direction = na::vector![0.0, 0.0, 0.0];
		for (idx, mean) in means.into_iter().enumerate() {
			if mean.1 == 0 {
				continue;
			}
			let mean = mean.0 / mean.1 as f32;
			let height = min + (idx as f32 + 0.5) * layer_height;
			let mean = na::vector![mean.x, height, mean.y];
			direction += (mean - origin.coords).normalize();
		}
		direction = direction.normalize();

		Self {
			origin,
			direction,
			algorithm: TrunkAxisAlgorithm::AverageDirection(layers),

			render: Some(TrunkAxisRender::new(origin, direction, max - min, state)),
		}
	}

	/// Get lowest and highest slice, calculate means and connect
	pub fn new_low_high_layer(
		points: &[na::Point3<f32>],
		classifications: &[Classification],
		layer_width: f32,
		state: &render::State,
	) -> Self {
		let points = points
			.iter()
			.zip(classifications)
			.filter_map(|(&p, &c)| (c == Classification::Trunk).then_some(p))
			.collect::<Vec<_>>();

		if points.is_empty() {
			log::warn!("No points for trunk");
			return Self::new_empty(TrunkAxisAlgorithm::LowHighLayer(layer_width));
		}
		let (mut min, mut max) = (points[0].y, points[0].y);
		for p in points[1..].iter().copied() {
			min = min.min(p.y);
			max = max.max(p.y);
		}

		let height = max - min;
		let layer = layer_width.min(height / 2.0);

		let mut low = (na::vector![0.0, 0.0], 0);
		let mut high = (na::vector![0.0, 0.0], 0);

		for p in points.iter().copied() {
			if p.y - min < layer {
				low.0 += na::vector![p.x, p.z];
				low.1 += 1;
			} else if max - p.y < layer {
				high.0 += na::vector![p.x, p.z];
				high.1 += 1;
			}
		}

		let low = low.0 / low.1 as f32;
		let high = high.0 / high.1 as f32;

		let origin = na::point![low.x, min, low.y];
		let target = na::point![high.x, max, high.y];
		let direction = (target - origin).normalize();

		Self {
			origin,
			direction,
			algorithm: TrunkAxisAlgorithm::LowHighLayer(layer_width),
			render: Some(TrunkAxisRender::new(origin, direction, max - min, state)),
		}
	}

	pub fn ui(&mut self, ui: &mut egui::Ui, segment: &SegmentData, state: &render::State) -> bool {
		let mut changed = false;
		ui.add_sized([ui.available_width(), 0.0], egui::Label::new("Trunk Axis"));
		if ui
			.add(egui::RadioButton::new(
				matches!(self.algorithm, TrunkAxisAlgorithm::None),
				"None",
			))
			.clicked()
		{
			self.algorithm = TrunkAxisAlgorithm::None;
			changed |= true;
		}
		if ui
			.add(egui::RadioButton::new(
				matches!(self.algorithm, TrunkAxisAlgorithm::AverageDirection(_)),
				"Average Direction",
			))
			.clicked()
		{
			self.algorithm = TrunkAxisAlgorithm::AverageDirection(10);
			changed |= true;
		}
		if ui
			.add(egui::RadioButton::new(
				matches!(self.algorithm, TrunkAxisAlgorithm::LowHighLayer(_)),
				"Low High Layer",
			))
			.clicked()
		{
			self.algorithm = TrunkAxisAlgorithm::LowHighLayer(1.0);
			changed |= true;
		}
		match &mut self.algorithm {
			TrunkAxisAlgorithm::None => {},
			TrunkAxisAlgorithm::AverageDirection(layers) => {
				ui.separator();
				ui.add_sized(
					[ui.available_width(), 0.0],
					egui::Label::new("Average Direction"),
				);
				egui::Grid::new(id!()).num_columns(2).show(ui, |ui| {
					ui.label("Slices");
					changed |= ui.add(egui::Slider::new(layers, 1..=16)).changed();
					ui.end_row();
				});
			},
			TrunkAxisAlgorithm::LowHighLayer(layers) => {
				ui.separator();
				ui.add_sized(
					[ui.available_width(), 0.0],
					egui::Label::new("Low High Layer"),
				);
				egui::Grid::new(id!()).num_columns(2).show(ui, |ui| {
					ui.label("Width");
					changed |= ui.add(egui::Slider::new(layers, 0.1..=5.0)).changed();
					ui.end_row();
				});
			},
		}
		if changed {
			self.update(segment, state);
		}
		changed
	}

	pub fn render<'a>(&'a self, lines_pass: &mut render::LinesPass<'a>) {
		if let Some(render) = &self.render {
			render.lines.render(&render.cloud, lines_pass);
		}
	}
}
