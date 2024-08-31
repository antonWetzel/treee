use nalgebra as na;

const BASE_ROTATE_SPEED: f32 = 0.002;
const VERTICAL_SPEED: f32 = 0.02;

/// 45 degrees
const FIELD_OF_VIEW: f32 = 45.0 * std::f32::consts::TAU / 360.0;

/// Camera controller
pub struct Camera {
	gpu: render::Camera3DGPU,
	cam: render::Camera3D,
	transform: na::Affine3<f32>,
	controller: Controller,
}

#[allow(dead_code)]
impl Camera {
	pub fn new(state: &render::State, aspect: f32) -> Self {
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
		}
	}

	pub fn update_aspect(&mut self, aspect: f32, state: &render::State) {
		self.cam.aspect = aspect;
		self.update_gpu(state);
	}

	pub fn update_gpu(&mut self, state: &render::State) {
		self.gpu = render::Camera3DGPU::new(state, &self.cam, &self.transform);
	}

	pub fn movement(&mut self, direction: na::Vector2<f32>, state: &render::State) {
		self.controller.movement(direction, &mut self.transform);
		self.update_gpu(state);
	}

	pub fn vertical_movement(&mut self, amount: f32, state: &render::State) {
		self.transform *= na::Translation3::new(
			0.0,
			amount * self.controller.distance() * VERTICAL_SPEED,
			0.0,
		);
		self.update_gpu(state);
	}

	pub fn move_in_view_direction(&mut self, amount: f32, state: &render::State) {
		self.transform *= na::Translation3::new(0.0, 0.0, amount);
		self.update_gpu(state);
	}

	pub fn rotate(&mut self, delta: na::Vector2<f32>, state: &render::State) {
		self.controller.rotate(delta, &mut self.transform);
		self.update_gpu(state);
	}

	pub fn scroll(&mut self, value: f32, state: &render::State) {
		self.controller.scroll(value, &mut self.transform);
		self.update_gpu(state);
	}

	pub fn position(&self) -> na::Point3<f32> {
		self.transform * na::Point3::origin()
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

	pub fn inside_moved_frustrum(
		&self,
		corner: na::Point3<f32>,
		size: f32,
		difference: f32,
	) -> bool {
		self.cam.inside(
			corner,
			size,
			self.transform * na::Translation3::new(0.0, 0.0, -difference),
		)
	}

	pub fn ray_origin(
		&self,
		_position: na::Point2<f32>,
		_window_size: na::Point2<f32>,
	) -> na::Point3<f32> {
		self.transform * na::Point::origin()
	}

	pub fn ray_direction(
		&self,
		position: na::Point2<f32>,
		window_size: na::Point2<f32>,
	) -> na::Vector3<f32> {
		let dist = (window_size.y / 2.0) / (FIELD_OF_VIEW / 2.0).tan();
		let position = position - window_size / 2.0;
		(self.transform * na::vector![position.x, -position.y, -dist]).normalize()
	}

	pub fn gpu(&self) -> &render::Camera3DGPU {
		&self.gpu
	}
}

#[derive(Clone, Copy, PartialEq)]
pub enum Controller {
	#[allow(dead_code)]
	FirstPerson {
		sensitivity: f32,
	},
	Orbital {
		offset: f32,
	},
}

impl Controller {
	pub fn distance(&self) -> f32 {
		match *self {
			Self::FirstPerson { sensitivity } => sensitivity,
			Self::Orbital { offset } => offset,
		}
	}

	pub fn movement(&mut self, direction: na::Vector2<f32>, transform: &mut na::Affine3<f32>) {
		match *self {
			Self::FirstPerson { sensitivity } => {
				*transform *=
					na::Translation3::new(direction.x * sensitivity, 0.0, direction.y * sensitivity)
			},
			Self::Orbital { offset } => {
				let vector = (*transform * na::vector![1.0, 0.0, 0.0] * direction.x
					+ (*transform * na::vector![1.0, 0.0, 0.0]).cross(&na::vector![0.0, 1.0, 0.0])
						* direction.y) * offset;
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
						&na::Unit::new_unchecked(na::vector![0.0, 1.0, 0.0]),
						delta.x * -BASE_ROTATE_SPEED,
					) * na::Translation3 { vector: -p.coords }
					* *transform * na::Rotation3::from_axis_angle(
					&na::Unit::new_unchecked(na::vector![1.0, 0.0, 0.0]),
					delta.y * -BASE_ROTATE_SPEED,
				);
			},
			Self::Orbital { offset } => {
				let d = *transform * (na::Point3::origin() + na::vector![0.0, 0.0, -1.0] * offset);
				*transform = na::Translation3 { vector: d.coords }
					* na::Rotation3::from_axis_angle(
						&na::Unit::new_unchecked(na::vector![0.0, 1.0, 0.0]),
						delta.x * -BASE_ROTATE_SPEED,
					) * na::Translation3 { vector: -d.coords }
					* *transform * na::Translation3::new(0.0, 0.0, -offset)
					* na::Rotation3::from_axis_angle(
						&na::Unit::new_unchecked(na::vector![1.0, 0.0, 0.0]),
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
