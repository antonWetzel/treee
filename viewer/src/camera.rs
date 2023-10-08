use math::Angle;
use math::Transform;
use math::Vector;
use math::{X, Y, Z};

use crate::lod;
use crate::State;

const BASE_MOVE_SPEED: f32 = 0.1;
const BASE_ROTATE_SPEED: f32 = 0.002;

pub struct Camera {
	pub gpu: render::Camera3DGPU,
	pub cam: render::Camera3D,
	pub transform: Transform<3, f32>,
	controller: Controller,
	pub lod: lod::Mode,
}

impl Camera {
	pub fn new(mut position: Vector<3, f32>, state: &State) -> Self {
		let camera = render::Camera3D::new(1.0, 45.0, 0.1, 10_000.0);
		let controller = Controller::Orbital { offset: 10.0 };
		position[Z] += 10.0;
		let transform = Transform::translation(position);

		Self {
			gpu: render::Camera3DGPU::new(state, &camera, &transform),
			transform,
			cam: camera,
			controller,
			lod: lod::Mode::Normal { threshold: 1.0 },
		}
	}

	pub fn movement(&mut self, direction: Vector<2, f32>, state: &State) {
		self.controller.movement(direction, &mut self.transform);
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}

	pub fn rotate(&mut self, delta: Vector<2, f64>, state: &State) {
		self.controller.rotate(delta, &mut self.transform);
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}

	pub fn scroll(&mut self, value: f32, state: &State) {
		self.controller.scroll(value, &mut self.transform);
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}

	pub fn position(&self) -> Vector<3, f32> {
		self.transform.position
	}

	pub fn change_lod(&mut self, max_level: usize) {
		self.lod = match &self.lod {
			lod::Mode::Level { .. } => lod::Mode::Normal { threshold: 1.0 },
			lod::Mode::Normal { .. } => lod::Mode::Level { target: 0, max: max_level },
		};
	}

	pub fn change_controller(&mut self) {
		self.controller = match &self.controller {
			Controller::FirstPerson { .. } => Controller::Orbital { offset: 10.0 },
			Controller::Orbital { .. } => Controller::FirstPerson { sensitivity: 10.0 },
		}
	}

	pub fn inside_frustrum(&self, corner: Vector<3, f32>, size: f32) -> bool {
		self.inside_frustrum_center(corner, size, self.position())
	}

	pub fn inside_moved_frustrum(&self, corner: Vector<3, f32>, size: f32, difference: f32) -> bool {
		let center = self.position() - self.transform.basis[Z] * difference;
		self.inside_frustrum_center(corner, size, center)
	}

	fn inside_frustrum_center(&self, corner: Vector<3, f32>, size: f32, center: Vector<3, f32>) -> bool {
		let offset = corner - center;
		let points: [Vector<3, f32>; 8] = [
			offset + [0.0, 0.0, 0.0].into(),
			offset + [0.0, 0.0, size].into(),
			offset + [0.0, size, 0.0].into(),
			offset + [0.0, size, size].into(),
			offset + [size, 0.0, 0.0].into(),
			offset + [size, 0.0, size].into(),
			offset + [size, size, 0.0].into(),
			offset + [size, size, size].into(),
		];
		let screen_y = (self.cam.fovy / 2.0).tan();
		let screen_x = screen_y * self.cam.aspect;
		let x_dir = self.transform.basis[X];
		let y_dir = self.transform.basis[Y];
		let z_dir = -self.transform.basis[Z];
		let normals = [
			(z_dir + x_dir * screen_x).cross(y_dir),
			(z_dir - x_dir * screen_x).cross(-y_dir),
			(z_dir + y_dir * screen_y).cross(-x_dir),
			(z_dir - y_dir * screen_y).cross(x_dir),
		];
		for normal in normals {
			if !Self::inside_plane(normal, &points) {
				return false;
			}
		}
		true
	}

	fn inside_plane(normal: Vector<3, f32>, points: &[Vector<3, f32>; 8]) -> bool {
		for point in points {
			if point.dot(normal) <= 0.0 {
				return true;
			}
		}
		false
	}
}

enum Controller {
	FirstPerson { sensitivity: f32 },
	Orbital { offset: f32 },
}

impl Controller {
	pub fn movement(&mut self, direction: Vector<2, f32>, transform: &mut Transform<3, f32>) {
		match *self {
			Controller::FirstPerson { sensitivity } => {
				let direction = [
					direction[X] * sensitivity * BASE_MOVE_SPEED,
					0.0,
					direction[Y] * sensitivity * BASE_MOVE_SPEED,
				]
				.into();
				*transform *= Transform::translation(direction);
			},
			Controller::Orbital { offset } => {
				transform.position += (transform.basis[X] * direction[X]
					+ transform.basis[X].cross([0.0, 1.0, 0.0].into()) * direction[Y])
					* offset * BASE_MOVE_SPEED;
			},
		}
	}

	pub fn rotate(&mut self, delta: Vector<2, f64>, transform: &mut Transform<3, f32>) {
		match *self {
			Controller::FirstPerson { .. } => {
				transform.rotate_local_before(
					[0.0, 1.0, 0.0].into(),
					Angle::radians(delta[X] as f32) * -BASE_ROTATE_SPEED,
				);
				transform.rotate_local(
					[1.0, 0.0, 0.0].into(),
					Angle::radians(delta[Y] as f32) * -BASE_ROTATE_SPEED,
				);
			},
			Controller::Orbital { offset } => {
				transform.position += transform.basis[Z] * -offset;
				transform.rotate_local_before(
					[0.0, 1.0, 0.0].into(),
					Angle::radians(delta[X] as f32) * -BASE_ROTATE_SPEED,
				);
				transform.rotate_local(
					[1.0, 0.0, 0.0].into(),
					Angle::radians(delta[Y] as f32) * -BASE_ROTATE_SPEED,
				);
				transform.position += transform.basis[Z] * offset;
			},
		}
	}

	pub fn scroll(&mut self, value: f32, transform: &mut Transform<3, f32>) {
		match self {
			Controller::FirstPerson { sensitivity } => *sensitivity *= 1.0 + value / 10.0,
			Controller::Orbital { offset } => {
				let new_offset = *offset * (1.0 + value / 10.0);
				transform.position -= transform.basis[Z] * (*offset - new_offset);
				*offset = new_offset;
			},
		}
	}
}
