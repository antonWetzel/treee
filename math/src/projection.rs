use std::ops::{Add, Div, Mul, Neg, Sub};

use crate::{
	matrix::Matrix,
	requirements::{FromF64, Identity, Trigonometry, Zero},
	vector::Vector,
};

pub struct Projection;

impl Projection {
	pub fn create_perspective<T>(vertical_fov: T, aspect_ratio: T, near: T, far: T) -> Matrix<4, 4, T>
	where
		T: FromF64,
		T: Identity,
		T: Trigonometry,
		T: Copy,
		T: Zero,
		T: Neg<Output = T>,
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
		T: Div<T, Output = T>,
	{
		let fov_rad = vertical_fov * T::from_f64(std::f64::consts::TAU / 360.0);
		let focal_length = T::IDENTITY / T::tan(fov_rad * T::from_f64(0.5));

		let x: T = focal_length / aspect_ratio;
		let y = focal_length;
		let norm: T = T::IDENTITY / (near - far);
		let c0 = (far + near) * norm;
		let c1 = (T::from_f64(2.0) * far * near) * norm;
		Matrix::<4, 4, T>::from([
			Vector::from([x, T::ZERO, T::ZERO, T::ZERO]),
			Vector::from([T::ZERO, y, T::ZERO, T::ZERO]),
			Vector::from([T::ZERO, T::ZERO, c0, -T::IDENTITY]),
			Vector::from([T::ZERO, T::ZERO, c1, T::ZERO]),
		])
	}
}
