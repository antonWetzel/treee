use std::{
	mem::MaybeUninit,
	ops::{Add, Index, IndexMut, Mul, MulAssign},
};

use crate::{requirements::Zero, vector::Vector, Dimension, Dimension2D, Dimensions};

#[derive(Clone, Copy, Debug)]
pub struct Matrix<const A: usize, const B: usize, T>([Vector<B, T>; A]);

impl<const A: usize, const B: usize, T> std::default::Default for Matrix<A, B, T>
where
	T: Zero,
{
	fn default() -> Self {
		let mut data: [Vector<B, T>; A] = unsafe { MaybeUninit::uninit().assume_init() };
		for value in data.iter_mut() {
			*value = <Vector<B, T>>::default();
		}
		Self(data)
	}
}

impl<const A: usize, const B: usize, T> Add for Matrix<A, B, T>
where
	T: Add<T, Output = T>,
	T: Copy,
{
	type Output = Self;

	fn add(mut self, other: Self) -> Self {
		for i in Dimensions(0..A) {
			self[i] += other[i];
		}
		self
	}
}

impl<const A: usize, const B: usize, const C: usize, T> Mul<Matrix<C, A, T>> for Matrix<A, B, T>
where
	T: Zero,
	T: Copy,
	T: Add<T, Output = T>,
	T: Mul<T, Output = T>,
{
	type Output = Matrix<C, B, T>;

	fn mul(self, other: Matrix<C, A, T>) -> Matrix<C, B, T> {
		let mut res = Matrix::<C, B, T>::default();
		for i in Dimensions(0..C) {
			for j in Dimensions(0..B) {
				for c in Dimensions(0..A) {
					res[i + j] = res[i + j] + self[c + j] * other[i + c];
				}
			}
		}
		res
	}
}

impl<const A: usize, const B: usize, T> Mul<Vector<A, T>> for Matrix<A, B, T>
where
	T: Zero,
	T: Copy,
	T: Add<T, Output = T>,
	T: Mul<T, Output = T>,
{
	type Output = Vector<B, T>;

	fn mul(self, other: Vector<A, T>) -> Vector<B, T> {
		let mut res = Vector::<B, T>::default();
		for i in Dimensions(0..B) {
			for c in Dimensions(0..A) {
				res[i] = res[i] + self[c + i] * other[c];
			}
		}
		res
	}
}

impl<const A: usize, const B: usize, T> MulAssign<T> for Matrix<A, B, T>
where
	T: Copy,
	T: Mul<T, Output = T>,
{
	fn mul_assign(&mut self, other: T) {
		for i in Dimensions(0..A) {
			for j in Dimensions(0..B) {
				self[i + j] = self[i + j] * other;
			}
		}
	}
}

impl<const A: usize, const B: usize, T> Mul<T> for Matrix<A, B, T>
where
	T: Copy,
	T: Mul<T, Output = T>,
{
	type Output = Matrix<A, B, T>;

	fn mul(mut self, other: T) -> Matrix<A, B, T> {
		self *= other;
		self
	}
}

impl<const A: usize, const B: usize, T> Matrix<A, B, T> {
	pub fn new(data: [Vector<B, T>; A]) -> Self {
		Self(data)
	}

	pub fn data(&self) -> [Vector<B, T>; A]
	where
		T: Copy,
	{
		self.0
	}
}

impl<const A: usize, const B: usize, T> Index<Dimension> for Matrix<A, B, T> {
	type Output = Vector<B, T>;

	fn index(&self, index: Dimension) -> &Vector<B, T> {
		&self.0[index.0]
	}
}

impl<const A: usize, const B: usize, T> IndexMut<Dimension> for Matrix<A, B, T> {
	fn index_mut(&mut self, index: Dimension) -> &mut Vector<B, T> {
		&mut self.0[index.0]
	}
}

impl<const A: usize, const B: usize, T> Index<Dimension2D> for Matrix<A, B, T> {
	type Output = T;

	fn index(&self, index: Dimension2D) -> &T {
		&self[Dimension(index.0)][Dimension(index.1)]
	}
}

impl<const A: usize, const B: usize, T> IndexMut<Dimension2D> for Matrix<A, B, T> {
	fn index_mut(&mut self, index: Dimension2D) -> &mut T {
		&mut self[Dimension(index.0)][Dimension(index.1)]
	}
}

impl<const A: usize, const B: usize, T: std::fmt::Display> std::fmt::Display for Matrix<A, B, T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "[")?;
		for j in Dimensions(0..B) {
			for i in Dimensions(0..A) {
				write!(f, "{:}\t", self[i + j])?;
			}
			if j.0 < (B - 1) {
				writeln!(f)?;
			} else {
				write!(f, "]")?;
			}
		}
		Ok(())
	}
}

impl<const A: usize, const B: usize, T> From<[Vector<B, T>; A]> for Matrix<A, B, T> {
	fn from(value: [Vector<B, T>; A]) -> Self {
		Self(value)
	}
}
