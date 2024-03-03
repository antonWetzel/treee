use std::fs::File;

use math::Angle;
use math::Transform;
use math::Vector;
use math::{X, Y, Z};
use serde::{Deserialize, Serialize};

use crate::lod;
use crate::State;

const BASE_MOVE_SPEED: f32 = 0.1;
const BASE_ROTATE_SPEED: f32 = 0.002;
const FIELD_OF_VIEW: f32 = 45.0;

pub struct Camera {
	pub gpu: render::Camera3DGPU,
	pub cam: render::Camera3D,
	pub transform: Transform<3, f32>,
	pub controller: Controller,
	pub lod: lod::Mode,
}

impl Camera {
	pub fn new(state: &State, aspect: f32) -> Self {
		let camera = render::Camera3D::Perspective {
			aspect,
			fovy: FIELD_OF_VIEW,
			near: 0.1,
			far: 10_000.0,
		};

		// todo: toggle switch
		// let camera = render::Camera3D::Orthographic {
		// 	aspect,
		// 	height: 1000.0,
		// 	near: 0.1,
		// 	far: 10_000.0,
		// };
		let controller = Controller::Orbital { offset: 100.0 };
		let position = [0.0, 0.0, 100.0].into();
		let transform = Transform::translation(position);

		Self {
			gpu: render::Camera3DGPU::new(state, &camera, &transform),
			transform,
			cam: camera,
			controller,
			lod: lod::Mode::new_auto(),
		}
	}

	pub fn movement(&mut self, direction: Vector<2, f32>, state: &State) {
		self.controller.movement(direction, &mut self.transform);
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}

	pub fn move_in_view_direction(&mut self, amount: f32, state: &State) {
		self.transform.position += self.transform.basis[Z] * amount;
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}

	pub fn rotate(&mut self, delta: Vector<2, f32>, state: &State) {
		self.controller.rotate(delta, &mut self.transform);
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}

	pub fn scroll(&mut self, value: f32, state: &State) {
		match &mut self.cam {
			render::Camera3D::Perspective { .. } => self.controller.scroll(value, &mut self.transform),
			render::Camera3D::Orthographic { height, .. } => *height *= 1.0 + value / 10.0,
		}
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}

	pub fn position(&self) -> Vector<3, f32> {
		self.transform.position
	}

	pub fn time(&mut self, render_time: f32) {
		let fps = 1.0 / render_time;
		match &mut self.lod {
			lod::Mode::Auto { threshold, target } => {
				if fps < *target / 1.25 {
					*threshold = (*threshold / 1.01).max(0.01);
				} else if fps > *target * 1.25 {
					*threshold = (*threshold * 1.01).min(10.0);
				}
			},
			_ => {},
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

	pub fn inside_frustrum(&self, corner: Vector<3, f32>, size: f32) -> bool {
		self.cam.inside(corner, size, self.transform)
	}

	pub fn inside_moved_frustrum(&self, corner: Vector<3, f32>, size: f32, difference: f32) -> bool {
		self.cam.inside(
			corner,
			size,
			self.transform * Transform::translation([0.0, 0.0, -difference].into()),
		)
	}

	pub fn ray_origin(&self, position: Vector<2, f32>, window_size: Vector<2, f32>) -> Vector<3, f32> {
		match self.cam {
			render::Camera3D::Perspective { .. } => self.transform.position,
			render::Camera3D::Orthographic { aspect, height, .. } => {
				self.transform.position
					+ self.transform.basis[X] * ((position[X] / window_size[X]) - 0.5) * (height * aspect)
					- self.transform.basis[Y] * ((position[Y] / window_size[Y]) - 0.5) * height
			},
		}
	}

	pub fn ray_direction(&self, position: Vector<2, f32>, window_size: Vector<2, f32>) -> Vector<3, f32> {
		match self.cam {
			render::Camera3D::Perspective { .. } => {
				let dist = window_size[Y] / (2.0 * Angle::degree(FIELD_OF_VIEW / 2.0).as_radians().tan());
				let position = position - window_size / 2.0;
				let intersection = -self.transform.basis[Z] * dist
					+ self.transform.basis[X] * position[X]
					+ -self.transform.basis[Y] * position[Y];
				intersection.normalized()
			},
			render::Camera3D::Orthographic { .. } => -self.transform.basis[Z],
		}
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
		bincode::serialize_into(file, &data).unwrap();
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
		let data = bincode::deserialize_from::<_, CameraData>(file).unwrap();

		self.transform = data.transform;
		self.controller = data.controller;
		self.lod = data.lod;
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}
}

#[derive(Serialize, Deserialize)]
struct CameraData {
	transform: Transform<3, f32>,
	controller: Controller,
	lod: lod::Mode,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Controller {
	FirstPerson { sensitivity: f32 },
	Orbital { offset: f32 },
}

impl Controller {
	pub fn movement(&mut self, direction: Vector<2, f32>, transform: &mut Transform<3, f32>) {
		match *self {
			Self::FirstPerson { sensitivity } => {
				let direction = [
					direction[X] * sensitivity * BASE_MOVE_SPEED,
					0.0,
					direction[Y] * sensitivity * BASE_MOVE_SPEED,
				]
				.into();
				*transform *= Transform::translation(direction);
			},
			Self::Orbital { offset } => {
				transform.position += (transform.basis[X] * direction[X]
					+ transform.basis[X].cross([0.0, 1.0, 0.0].into()) * direction[Y])
					* offset * BASE_MOVE_SPEED;
			},
		}
	}

	pub fn rotate(&mut self, delta: Vector<2, f32>, transform: &mut Transform<3, f32>) {
		match *self {
			Self::FirstPerson { .. } => {
				transform.rotate_local_before(
					[0.0, 1.0, 0.0].into(),
					Angle::radians(delta[X]) * -BASE_ROTATE_SPEED,
				);
				transform.rotate_local(
					[1.0, 0.0, 0.0].into(),
					Angle::radians(delta[Y]) * -BASE_ROTATE_SPEED,
				);
			},
			Self::Orbital { offset } => {
				transform.position += transform.basis[Z] * -offset;
				transform.rotate_local_before(
					[0.0, 1.0, 0.0].into(),
					Angle::radians(delta[X]) * -BASE_ROTATE_SPEED,
				);
				transform.rotate_local(
					[1.0, 0.0, 0.0].into(),
					Angle::radians(delta[Y]) * -BASE_ROTATE_SPEED,
				);
				transform.position += transform.basis[Z] * offset;
			},
		}
	}

	pub fn scroll(&mut self, value: f32, transform: &mut Transform<3, f32>) {
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
				transform.position -= transform.basis[Z] * (*offset - new_offset);
				*offset = new_offset;
			},
		}
	}
}
