use std::ops::{Add, Div, Mul, MulAssign, Neg, Sub};

use crate::{
	angle::Angle,
	mat::Mat,
	matrix::Matrix,
	requirements::{FromF64, Identity, Trigonometry, Zero},
	vector::Vector,
	Dimensions, X, Y, Z,
};

#[derive(Clone, Copy, Debug)]
pub struct Transform<const N: usize, T> {
	pub basis:    Mat<N, T>,
	pub position: Vector<N, T>,
}

impl<const N: usize, T> Default for Transform<N, T>
where
	T: Zero,
	T: Copy,
{
	fn default() -> Self {
		Self {
			basis:    Mat::<N, T>::default(),
			position: Vector::<N, T>::default(),
		}
	}
}

impl<const N: usize, T> Transform<N, T> {
	pub fn identity() -> Self
	where
		T: Identity,
		T: Zero,
		T: Copy,
	{
		Self {
			basis:    Mat::<N, T>::identity(),
			position: Vector::<N, T>::default(),
		}
	}

	pub fn translation(position: Vector<N, T>) -> Self
	where
		T: Identity,
		T: Zero,
	{
		Self { basis: Mat::<N, T>::identity(), position }
	}
	pub fn scale(scale: Vector<N, T>) -> Self
	where
		T: Identity,
		T: Zero,
		T: Copy,
		T: Mul<T, Output = T>,
	{
		let mut basis = Mat::<N, T>::identity();
		for i in Dimensions(0..N) {
			basis[i] *= scale[i];
		}
		Self { basis, position: Vector::default() }
	}
}

impl<T> Transform<2, T>
where
	T: Zero + Identity,
{
	pub fn as_matrix(self) -> Mat<3, T>
	where
		T: Identity,
		T: Zero,
		T: Copy,
	{
		[
			[self.basis[X + X], self.basis[X + Y], T::ZERO].into(),
			[self.basis[Y + X], self.basis[Y + Y], T::ZERO].into(),
			[self.position[X], self.position[Y], T::IDENTITY].into(),
		]
		.into()
	}
}

impl<T> Transform<3, T>
where
	T: Zero + Identity,
{
	pub fn as_matrix(self) -> Mat<4, T>
	where
		T: Identity,
		T: Zero,
		T: Copy,
	{
		[
			[
				self.basis[X + X],
				self.basis[X + Y],
				self.basis[X + Z],
				T::ZERO,
			]
			.into(),
			[
				self.basis[Y + X],
				self.basis[Y + Y],
				self.basis[Y + Z],
				T::ZERO,
			]
			.into(),
			[
				self.basis[Z + X],
				self.basis[Z + Y],
				self.basis[Z + Z],
				T::ZERO,
			]
			.into(),
			[
				self.position[X],
				self.position[Y],
				self.position[Z],
				T::IDENTITY,
			]
			.into(),
		]
		.into()
	}
}

impl<const N: usize, T> Mul<Vector<N, T>> for Transform<N, T>
where
	T: Zero,
	T: Copy,
	T: Add<T, Output = T>,
	T: Mul<T, Output = T>,
{
	type Output = Vector<N, T>;

	fn mul(self, other: Vector<N, T>) -> Vector<N, T> {
		self.basis * other + self.position
	}
}

impl<const N: usize, T> Mul<Self> for Transform<N, T>
where
	T: Zero,
	T: Copy,
	T: Add<T, Output = T>,
	T: Mul<T, Output = T>,
{
	type Output = Self;

	fn mul(self, rhs: Self) -> Self::Output {
		let basis = self.basis * rhs.basis;
		let mut position = self.position;
		for i in Dimensions(0..N) {
			position += self.basis[i] * rhs.position[i];
		}
		Self { position, basis }
	}
}

impl<const N: usize, T> MulAssign for Transform<N, T>
where
	T: Zero,
	T: Copy,
	T: Add<T, Output = T>,
	T: Mul<T, Output = T>,
{
	fn mul_assign(&mut self, rhs: Self) {
		*self = *self * rhs;
	}
}

impl<T> Transform<2, T> {
	pub fn inverse(self) -> Self
	where
		T: Identity,
		T: Zero,
		T: Copy,
		T: Neg<Output = T>,
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
		T: Div<T, Output = T>,
	{
		let inv = self.basis.inverse();
		let pos = -(inv * self.position);
		Self { basis: inv, position: pos }
	}

	pub fn padded_matrix(self) -> Matrix<3, 4, T>
	where
		T: Zero,
		T: Copy,
		T: Identity,
	{
		[
			[self.basis[X + X], self.basis[X + Y], T::ZERO, T::ZERO].into(),
			[self.basis[Y + X], self.basis[Y + Y], T::ZERO, T::ZERO].into(),
			[self.position[X], self.position[Y], T::IDENTITY, T::ZERO].into(),
		]
		.into()
	}

	pub fn rotatation(angle: Angle<T>) -> Self
	where
		T: FromF64,
		T: Copy,
		T: Trigonometry,
		T: Zero,
		T: Neg<Output = T>,
		T: Mul<T, Output = T>,
	{
		Self {
			basis:    Mat::<2, T>::rotation(angle),
			position: Vector::default(),
		}
	}

	pub fn rotate_local(&mut self, angle: Angle<T>)
	where
		T: FromF64,
		T: Trigonometry,
		T: Zero,
		T: Copy,
		T: Neg<Output = T>,
		T: Add<T, Output = T>,
		T: Mul<T, Output = T>,
	{
		self.basis *= Mat::<2, T>::rotation(angle);
	}

	pub fn rotate_world(&mut self, angle: Angle<T>)
	where
		T: FromF64,
		T: Trigonometry,
		T: Copy,
		T: Zero,
		T: Neg<Output = T>,
		T: Add<T, Output = T>,
		T: Mul<T, Output = T>,
	{
		*self = Self::rotatation(angle) * *self;
	}
}

impl<T> Transform<3, T> {
	pub fn inverse(self) -> Self
	where
		T: Identity,
		T: Copy,
		T: Zero,
		T: Neg<Output = T>,
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
		T: Div<T, Output = T>,
	{
		let inv = self.basis.inverse();
		let pos = -(inv * self.position);
		Self { basis: inv, position: pos }
	}

	pub fn rotatation(axis: Vector<3, T>, angle: Angle<T>) -> Self
	where
		T: Zero,
		T: Identity,
		T: FromF64,
		T: Trigonometry,
		T: Copy,
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
	{
		Self {
			basis:    Mat::<3, T>::rotation(axis, angle),
			position: Vector::default(),
		}
	}

	pub fn rotate_local(&mut self, axis: Vector<3, T>, angle: Angle<T>)
	where
		T: Zero,
		T: Copy,
		T: Identity,
		T: FromF64,
		T: Trigonometry,
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
	{
		self.basis *= Mat::<3, T>::rotation(axis, angle);
	}

	// todo: better name
	pub fn rotate_local_before(&mut self, axis: Vector<3, T>, angle: Angle<T>)
	where
		T: Zero,
		T: Identity,
		T: FromF64,
		T: Trigonometry,
		T: Copy,
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
	{
		self.basis = Mat::<3, T>::rotation(axis, angle) * self.basis;
	}

	pub fn rotate_world(&mut self, axis: Vector<3, T>, angle: Angle<T>)
	where
		T: Zero,
		T: Identity,
		T: FromF64,
		T: Trigonometry,
		T: Copy,
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
	{
		*self = Self::rotatation(axis, angle) * *self;
	}
}
