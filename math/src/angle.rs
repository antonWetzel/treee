use std::ops::Mul;

use crate::requirements::FromF64;

#[derive(Clone, Copy, Debug)]
pub struct Angle<T>(T);

impl<T> Angle<T>
where
	T: FromF64,
	T: Mul<T, Output = T>,
{
	const DEG_TO_RAD: f64 = std::f64::consts::TAU / 360.0;
	const RAD_TO_DEG: f64 = 1.0 / Self::DEG_TO_RAD;

	pub fn degree(value: T) -> Angle<T> {
		Angle(value * T::from_f64(Self::DEG_TO_RAD))
	}

	pub fn radians(value: T) -> Angle<T> {
		Angle(value)
	}

	pub fn as_radians(self) -> T {
		self.0
	}

	pub fn as_degrees(self) -> T {
		self.0 * T::from_f64(Self::RAD_TO_DEG)
	}
}

impl<T> std::ops::Mul<T> for Angle<T>
where
	T: Mul<T, Output = T>,
{
	type Output = Angle<T>;

	fn mul(self, rhs: T) -> Self::Output {
		Self(self.0 * rhs)
	}
}
