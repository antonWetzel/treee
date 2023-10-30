use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub};

use crate::{
	angle::Angle,
	matrix::Matrix,
	requirements::{FromF64, Identity, PowI, Sqrt, Trigonometry, Zero},
	vector::Vector,
	Dimensions, X, Y, Z,
};

pub type Mat<const N: usize, T> = Matrix<N, N, T>;

impl<const N: usize, T> Mat<N, T>
where
	T: Identity,
	T: Zero,
{
	pub fn identity() -> Self {
		let mut res = Self::default();
		for i in Dimensions(0..N) {
			res[i + i] = T::IDENTITY;
		}
		res
	}
}

impl<const N: usize, T> MulAssign<Mat<N, T>> for Mat<N, T>
where
	T: Zero,
	T: Copy,
	T: Add<T, Output = T>,
	T: Mul<T, Output = T>,
{
	fn mul_assign(&mut self, other: Mat<N, T>) {
		*self = *self * other;
	}
}

impl<T> Mat<2, T>
where
	T: Identity,
	T: Copy,
	T: Neg<Output = T>,
	T: Sub<T, Output = T>,
	T: Mul<T, Output = T>,
	T: Div<T, Output = T>,
{
	pub fn inverse(self) -> Self {
		let det = self[X + X] * self[Y + Y] - self[Y + X] * self[X + Y];
		Self::from([
			[self[Y + Y], -self[X + Y]].into(),
			[-self[Y + X], self[X + X]].into(),
		]) * (T::IDENTITY / det)
	}
}

impl<T> Mat<2, T>
where
	T: FromF64,
	T: Copy,
	T: Trigonometry,
	T: Neg<Output = T>,
	T: Mul<T, Output = T>,
{
	pub fn rotation(angle: Angle<T>) -> Self {
		let a = angle.as_radians();
		let sin = T::sin(a);
		let cos = T::cos(a);
		// https://en.wikipedia.org/wiki/Rotation_matrix
		[[cos, sin].into(), [-sin, cos].into()].into()
	}
}

impl<T> Mat<3, T> {
	pub fn inverse(self) -> Self
	where
		T: Identity,
		T: Copy,
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
		T: Div<T, Output = T>,
	{
		let det = self[X + X] * (self[Y + Y] * self[Z + Z] - self[Z + Y] * self[Y + Z])
			- self[X + Y] * (self[Y + X] * self[Z + Z] - self[Y + Z] * self[Z + X])
			+ self[X + Z] * (self[Y + X] * self[Z + Y] - self[Y + Y] * self[Z + X]);
		Self::from([
			[
				(self[Y + Y] * self[Z + Z] - self[Z + Y] * self[Y + Z]),
				(self[X + Z] * self[Z + Y] - self[X + Y] * self[Z + Z]),
				(self[X + Y] * self[Y + Z] - self[X + Z] * self[Y + Y]),
			]
			.into(),
			[
				(self[Y + Z] * self[Z + X] - self[Y + X] * self[Z + Z]),
				(self[X + X] * self[Z + Z] - self[X + Z] * self[Z + X]),
				(self[Y + X] * self[X + Z] - self[X + X] * self[Y + Z]),
			]
			.into(),
			[
				(self[Y + X] * self[Z + Y] - self[Z + X] * self[Y + Y]),
				(self[Z + X] * self[X + Y] - self[X + X] * self[Z + Y]),
				(self[X + X] * self[Y + Y] - self[Y + X] * self[X + Y]),
			]
			.into(),
		]) * (T::IDENTITY / det)
	}

	pub fn raw(self) -> [[T; 4]; 3]
	where
		T: Copy,
		T: Zero,
	{
		[
			[self[X + X], self[X + Y], self[X + Z], T::ZERO],
			[self[Y + X], self[Y + Y], self[Y + Z], T::ZERO],
			[self[Z + X], self[Z + Y], self[Z + Z], T::ZERO],
		]
	}

	pub fn rotation(axis: Vector<3, T>, angle: Angle<T>) -> Self
	where
		T: Identity,
		T: FromF64,
		T: Trigonometry,
		T: Copy,
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
	{
		let a = angle.as_radians();
		let sin = T::sin(a);
		let cos = T::cos(a);
		let neg_cos = T::IDENTITY - cos;
		let x = axis[X];
		let y = axis[Y];
		let z = axis[Z];
		// https://en.wikipedia.org/wiki/Rotation_matrix
		[
			[
				x * x * neg_cos + cos,
				y * x * neg_cos + z * sin,
				z * x * neg_cos - y * sin,
			]
			.into(),
			[
				x * y * neg_cos - z * sin,
				y * y * neg_cos + cos,
				z * y * neg_cos + x * sin,
			]
			.into(),
			[
				x * z * neg_cos + y * sin,
				y * z * neg_cos - x * sin,
				z * z * neg_cos + cos,
			]
			.into(),
		]
		.into()
	}

	pub fn determinant(&self) -> T
	where
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
		T: Copy,
		T: PartialEq,
		T: Sqrt,
	{
		self[X + X] * (self[Y + Y] * self[Z + Z] - self[Z + Y] * self[Y + Z])
			- self[X + Y] * (self[Y + X] * self[Z + Z] - self[Y + Z] * self[Z + X])
			+ self[X + Z] * (self[Y + X] * self[Z + Y] - self[Y + Y] * self[Z + X])
	}

	// https://en.wikipedia.org/wiki/Eigenvalue_algorithm#3%C3%973_matrices
	// the matrix must be real and symmetric
	pub fn fast_eigenvalues(&self) -> Vector<3, T>
	where
		T: Identity,
		T: FromF64,
		T: Trigonometry,
		T: Copy,
		T: Zero,
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
		T: Div<T, Output = T>,
		T: Sqrt,
		T: PowI,
		T: PartialOrd,
	{
		fn square<T: std::ops::Mul<Output = T> + Copy>(x: T) -> T {
			x * x
		}
		// I would choose better names for the variables if I know what they mean
		let p1 = square(self[X + Y]) + square(self[X + Z]) + square(self[Y + Z]);
		if p1 == T::ZERO {
			return [self[X + X], self[Y + Y], self[Z + Z]].into();
		}

		let q = (self[X + X] + self[Y + Y] + self[Z + Z]) / T::from_f64(3.0);
		let p2 = square(self[X + X] - q) + square(self[Y + Y] - q) + square(self[Z + Z] - q) + T::from_f64(2.0) * p1;
		let p = (p2 / T::from_f64(6.0)).sqrt();
		let mut mat_b = *self;
		for i in X.to(Z) {
			mat_b[i + i] = mat_b[i + i] - q;
		}
		let r = mat_b.determinant() / T::from_f64(2.0) * p.powi(-3);
		let phi = if r <= T::from_f64(-1.0) {
			T::from_f64(std::f64::consts::PI) / T::from_f64(3.0)
		} else if r >= T::IDENTITY {
			T::ZERO
		} else {
			r.acos() / T::from_f64(3.0)
		};

		let eig_1 = q + T::from_f64(2.0) * p * phi.cos();
		let eig_3 = q + T::from_f64(2.0)
			* p * (phi + (T::from_f64(2.0) * T::from_f64(std::f64::consts::PI) / T::from_f64(3.0))).cos();
		let eig_2 = T::from_f64(3.0) * q - eig_1 - eig_3;
		[eig_1, eig_2, eig_3].into()
	}

	pub fn calculate_last_eigenvector(&self, eigen_values: Vector<3, T>) -> Vector<3, T>
	where
		T: Zero,
		T: Copy,
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
		T: Div<T, Output = T>,
		T: AddAssign<T>,
		T: PartialEq,
		T: Sqrt,
	{
		let mut eigen_vector = Vector::<3, T>::default();
		for j in X.to(Z) {
			for k in X.to(Z) {
				eigen_vector[j] += (self[k + j] - if k == j { eigen_values[X] } else { T::ZERO })
					* (self[Z + k] - if Z == k { eigen_values[Y] } else { T::ZERO });
			}
		}
		eigen_vector / eigen_vector.length()
	}

	pub fn calculate_eigenvectors(&self, eigen_values: Vector<3, T>) -> Mat<3, T>
	where
		T: Zero,
		T: Copy,
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
		T: Div<T, Output = T>,
		T: AddAssign<T>,
		T: PartialEq,
		T: Sqrt,
	{
		let mut eigen_vectors = Mat::default();
		for i in X.to(Z) {
			let next = i.next(Z);
			let prev = i.previous(Z);
			for j in X.to(Z) {
				for k in X.to(Z) {
					let l = self[k + j] - if k == j { eigen_values[next] } else { T::ZERO };
					let r = self[i + k] - if i == k { eigen_values[prev] } else { T::ZERO };
					eigen_vectors[i + j] += l * r;
				}
			}
		}
		for i in X.to(Z) {
			eigen_vectors[i] = eigen_vectors[i].normalized();
		}
		eigen_vectors
	}
}
