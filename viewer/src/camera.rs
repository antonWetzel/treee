use std::fs::File;

use na::vector;
use nalgebra as na;
use serde::{Deserialize, Serialize};

use crate::lod;
use crate::State;

const BASE_MOVE_SPEED: f32 = 0.1;
const BASE_ROTATE_SPEED: f32 = 0.002;
const FIELD_OF_VIEW: f32 = 45.0 * std::f32::consts::TAU / 360.0;

pub struct Camera {
	pub gpu: render::Camera3DGPU,
	pub cam: render::Camera3D,
	pub transform: na::Affine3<f32>,
	pub controller: Controller,
	pub lod: lod::Mode,
}

impl Camera {
	pub fn new(state: &State, aspect: f32) -> Self {
		let camera = render::Camera3D {
			aspect,
			fovy: FIELD_OF_VIEW,
			near: 0.1,
			far: 10_000.0,
		};

		let controller = Controller::Orbital { offset: 100.0 };
		let transform = na::Affine3::identity() * na::Translation3::new(0.0, 0.0, 100.0);

		Self {
			gpu: render::Camera3DGPU::new(state, &camera, &transform),
			transform,
			cam: camera,
			controller,
			lod: lod::Mode::new_auto(),
		}
	}

	pub fn update_gpu(&mut self, state: &State) {
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}

	pub fn movement(&mut self, direction: na::Vector2<f32>, state: &State) {
		self.controller.movement(direction, &mut self.transform);
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}

	pub fn move_in_view_direction(&mut self, amount: f32, state: &State) {
		self.transform *= na::Translation3::new(0.0, 0.0, amount);
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}

	pub fn rotate(&mut self, delta: na::Vector2<f32>, state: &State) {
		self.controller.rotate(delta, &mut self.transform);
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}

	pub fn scroll(&mut self, value: f32, state: &State) {
		self.controller.scroll(value, &mut self.transform);
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}

	pub fn position(&self) -> na::Point3<f32> {
		self.transform * na::Point3::origin()
	}

	pub fn time(&mut self, render_time: f32) {
		let fps = 1.0 / render_time;
		if let lod::Mode::Auto { threshold, target } = &mut self.lod {
			if fps < *target / 1.25 {
				*threshold = (*threshold / 1.01).max(0.01);
			} else if fps > *target * 1.25 {
				*threshold = (*threshold * 1.01).min(10.0);
			}
		}
	}

	pub fn first_person(&self) -> Controller {
		match &self.controller {
			c @ Controller::FirstPerson { .. } => *c,
			Controller::Orbital { offset } => Controller::FirstPerson { sensitivity: *offset },
		}
	}

	pub fn orbital(&self) -> Controller {
		match &self.controller {
			Controller::FirstPerson { sensitivity } => Controller::Orbital { offset: *sensitivity },
			c @ Controller::Orbital { .. } => *c,
		}
	}

	pub fn inside_frustrum(&self, corner: na::Point3<f32>, size: f32) -> bool {
		self.cam.inside(corner, size, self.transform)
	}

	pub fn inside_moved_frustrum(&self, corner: na::Point3<f32>, size: f32, difference: f32) -> bool {
		self.cam.inside(
			corner,
			size,
			self.transform * na::Translation3::new(0.0, 0.0, -difference),
		)
	}

	pub fn ray_origin(&self, _position: na::Point2<f32>, _window_size: na::Point2<f32>) -> na::Point3<f32> {
		self.transform * na::Point::origin()
	}

	pub fn ray_direction(&self, position: na::Point2<f32>, window_size: na::Point2<f32>) -> na::Vector3<f32> {
		let dist = (window_size.y / 2.0) / (FIELD_OF_VIEW / 2.0).tan();
		let position = position - window_size / 2.0;
		(self.transform * vector![position.x, -position.y, -dist]).normalize()
	}

	pub fn save(&self) {
		let Some(path) = rfd::FileDialog::new()
			.add_filter("Camera Data", &["camdata"])
			.save_file()
		else {
			return;
		};
		let data = CameraData {
			transform: self.transform,
			controller: self.controller,
			lod: self.lod,
		};
		let Ok(file) = File::create(path) else {
			return;
		};
		serde_json::to_writer(file, &data).unwrap();
	}

	pub fn load(&mut self, state: &State) {
		let Some(path) = rfd::FileDialog::new()
			.add_filter("Camera Data", &["camdata"])
			.pick_file()
		else {
			return;
		};
		let Ok(file) = File::open(path) else {
			return;
		};
		let data = serde_json::from_reader::<_, CameraData>(file).unwrap();

		self.transform = data.transform;
		self.controller = data.controller;
		self.lod = data.lod;
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}
}

#[derive(Serialize, Deserialize)]
struct CameraData {
	transform: na::Affine3<f32>,
	controller: Controller,
	lod: lod::Mode,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Controller {
	FirstPerson { sensitivity: f32 },
	Orbital { offset: f32 },
}

impl Controller {
	pub fn movement(&mut self, direction: na::Vector2<f32>, transform: &mut na::Affine3<f32>) {
		match *self {
			Self::FirstPerson { sensitivity } => {
				*transform *= na::Translation3::new(
					direction.x * sensitivity * BASE_MOVE_SPEED,
					0.0,
					direction.y * sensitivity * BASE_MOVE_SPEED,
				)
			},
			Self::Orbital { offset } => {
				let vector = (*transform * vector![1.0, 0.0, 0.0] * direction.x
					+ (*transform * vector![1.0, 0.0, 0.0]).cross(&vector![0.0, 1.0, 0.0]) * direction.y)
					* offset * BASE_MOVE_SPEED;
				*transform = na::Translation3 { vector } * *transform;
			},
		}
	}

	pub fn rotate(&mut self, delta: na::Vector2<f32>, transform: &mut na::Affine3<f32>) {
		match *self {
			Self::FirstPerson { .. } => {
				let p = *transform * na::Point3::origin();

				*transform = na::Translation3 { vector: p.coords }
					* na::Rotation3::from_axis_angle(
						&na::Unit::new_unchecked(vector![0.0, 1.0, 0.0]),
						delta.x * -BASE_ROTATE_SPEED,
					) * na::Translation3 { vector: -p.coords }
					* *transform * na::Rotation3::from_axis_angle(
					&na::Unit::new_unchecked(vector![1.0, 0.0, 0.0]),
					delta.y * -BASE_ROTATE_SPEED,
				);
			},
			Self::Orbital { offset } => {
				let d = *transform * (na::Point3::origin() + vector![0.0, 0.0, -1.0] * offset);
				*transform = na::Translation3 { vector: d.coords }
					* na::Rotation3::from_axis_angle(
						&na::Unit::new_unchecked(vector![0.0, 1.0, 0.0]),
						delta.x * -BASE_ROTATE_SPEED,
					) * na::Translation3 { vector: -d.coords }
					* *transform * na::Translation3::new(0.0, 0.0, -offset)
					* na::Rotation3::from_axis_angle(
						&na::Unit::new_unchecked(vector![1.0, 0.0, 0.0]),
						delta.y * -BASE_ROTATE_SPEED,
					) * na::Translation3::new(0.0, 0.0, offset);
			},
		}
	}

	pub fn scroll(&mut self, value: f32, transform: &mut na::Affine3<f32>) {
		match self {
			Self::FirstPerson { sensitivity } => {
				*sensitivity *= 1.0 + value / 10.0;
				if *sensitivity < 0.01 {
					*sensitivity = 0.01;
				}
			},
			Self::Orbital { offset } => {
				let mut new_offset = *offset * (1.0 + value / 10.0);
				if new_offset < 0.01 {
					new_offset = 0.01;
				}
				*transform *= na::Translation3::new(0.0, 0.0, new_offset - *offset);
				*offset = new_offset;
			},
		}
	}
}
