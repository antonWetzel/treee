use std::ops::{Add, Mul, Sub};

use crate::{
	requirements::{FromF64, Identity},
	vector::Vector,
	X, Y, Z,
};

#[derive(Clone, Copy)]
pub struct Quaternion<T> {
	pub w: T,
	pub x: T,
	pub y: T,
	pub z: T,
}

impl<T> Mul<Vector<3, T>> for Quaternion<T>
where
	T: FromF64,
	T: Identity,
	T: Copy,
	T: Add<T, Output = T>,
	T: Sub<T, Output = T>,
	T: Mul<T, Output = T>,
{
	type Output = Vector<3, T>;

	fn mul(self, vec: Vector<3, T>) -> Self::Output {
		let num = self.x * T::from_f64(2.0);
		let num2 = self.y * T::from_f64(2.0);
		let num3 = self.z * T::from_f64(2.0);
		let num4 = self.x * num;
		let num5 = self.y * num2;
		let num6 = self.z * num3;
		let num7 = self.x * num2;
		let num8 = self.x * num3;
		let num9 = self.y * num3;
		let num10 = self.w * num;
		let num11 = self.w * num2;
		let num12 = self.w * num3;
		[
			(T::IDENTITY - (num5 + num6)) * vec[X] + (num7 - num12) * vec[Y] + (num8 + num11) * vec[Z],
			(num7 + num12) * vec[X] + (T::IDENTITY - (num4 + num6)) * vec[Y] + (num9 - num10) * vec[Z],
			(num8 - num11) * vec[X] + (num9 + num10) * vec[Y] + (T::IDENTITY - (num4 + num5)) * vec[Z],
		]
		.into()
	}
}
