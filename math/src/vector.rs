use std::{
	mem::MaybeUninit,
	ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign},
};

use serde::{Deserialize, Serialize};

use crate::{
	requirements::{Sqrt, Zero},
	Dimension, Dimensions, X, Y, Z,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Vector<const N: usize, T>([T; N]);

unsafe impl<const N: usize, T: bytemuck::Zeroable> bytemuck::Zeroable for Vector<N, T> {}
unsafe impl<const N: usize, T: bytemuck::Pod> bytemuck::Pod for Vector<N, T> {}

impl<const N: usize, T> std::default::Default for Vector<N, T>
where
	T: Zero,
{
	fn default() -> Self {
		let mut data: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };
		for value in data.iter_mut() {
			value.write(T::ZERO);
		}
		unsafe { (*(&data as *const _ as *const MaybeUninit<_>)).assume_init_read() }
	}
}

impl<const N: usize, T> Index<Dimension> for Vector<N, T> {
	type Output = T;

	fn index(&self, index: Dimension) -> &T {
		&self.0[index.0]
	}
}

impl<const N: usize, T> IndexMut<Dimension> for Vector<N, T> {
	fn index_mut(&mut self, index: Dimension) -> &mut T {
		&mut self.0[index.0]
	}
}

impl<const N: usize, T: Add<Output = T>> Add for Vector<N, T>
where
	T: Copy,
{
	type Output = Self;

	fn add(mut self, other: Self) -> Self {
		for i in Dimensions(0..N) {
			self[i] = self[i] + other[i];
		}
		self
	}
}

impl<const N: usize, T: Add<Output = T>> AddAssign for Vector<N, T>
where
	T: Copy,
{
	fn add_assign(&mut self, rhs: Self) {
		*self = *self + rhs;
	}
}

impl<const N: usize, T: Sub<Output = T>> Sub for Vector<N, T>
where
	T: Copy,
{
	type Output = Self;

	fn sub(mut self, other: Self) -> Self {
		for i in Dimensions(0..N) {
			self[i] = self[i] - other[i];
		}
		self
	}
}

impl<const N: usize, T: Sub<Output = T>> SubAssign for Vector<N, T>
where
	T: Copy,
{
	fn sub_assign(&mut self, rhs: Self) {
		*self = *self - rhs;
	}
}

impl<const N: usize, T: Neg<Output = T>> Neg for Vector<N, T>
where
	T: Copy,
{
	type Output = Self;

	fn neg(mut self) -> Self {
		for i in Dimensions(0..N) {
			self[i] = -self[i];
		}
		self
	}
}

impl<const N: usize, T: Mul<Output = T>> MulAssign<T> for Vector<N, T>
where
	T: Copy,
{
	fn mul_assign(&mut self, other: T) {
		for i in Dimensions(0..N) {
			self[i] = self[i] * other;
		}
	}
}

impl<const N: usize, T> Mul<T> for Vector<N, T>
where
	T: Mul<Output = T>,
	T: Copy,
{
	type Output = Self;

	fn mul(mut self, other: T) -> Self {
		self *= other;
		self
	}
}

impl<const N: usize, T: Div<Output = T>> DivAssign<T> for Vector<N, T>
where
	T: Copy,
{
	fn div_assign(&mut self, other: T) {
		for i in Dimensions(0..N) {
			self[i] = self[i] / other;
		}
	}
}

impl<const N: usize, T: Div<Output = T>> Div<T> for Vector<N, T>
where
	T: Copy,
{
	type Output = Self;

	fn div(mut self, other: T) -> Self {
		self /= other;
		self
	}
}

impl<const N: usize, T> Vector<N, T> {
	pub const fn new(data: [T; N]) -> Self {
		Self(data)
	}

	pub fn data(self) -> [T; N]
	where
		T: Copy,
	{
		self.0
	}

	pub fn data_ref(&self) -> &[T; N] {
		&self.0
	}

	pub fn data_mut(&mut self) -> &mut [T; N] {
		&mut self.0
	}
}

impl<const N: usize, T> Vector<N, T> {
	pub fn dot(self, other: Self) -> T
	where
		T: Zero,
		T: Copy,
		T: Add<T, Output = T>,
		T: Mul<T, Output = T>,
	{
		let mut res = T::ZERO;
		for i in Dimensions(0..N) {
			res = res + self[i] * other[i];
		}
		res
	}

	pub fn length_squared(self) -> T
	where
		T: Zero,
		T: Copy,
		T: Add<T, Output = T>,
		T: Mul<T, Output = T>,
		T: Sqrt,
	{
		self.dot(self)
	}

	pub fn length(self) -> T
	where
		T: Zero,
		T: Copy,
		T: Add<T, Output = T>,
		T: Mul<T, Output = T>,
		T: Sqrt,
	{
		self.length_squared().sqrt()
	}

	pub fn normalized(&self) -> Self
	where
		T: Zero,
		T: Copy,
		T: Add<T, Output = T>,
		T: Mul<T, Output = T>,
		T: Div<T, Output = T>,
		T: Sqrt,
	{
		*self / self.length()
	}

	pub fn map<U: Zero>(self, f: impl Fn(T) -> U) -> Vector<N, U> {
		Vector(self.0.map(f))
	}

	pub fn max(mut self, other: Vector<N, T>) -> Vector<N, T>
	where
		T: PartialOrd + Copy,
	{
		for i in Dimensions(0..N) {
			if other[i] > self[i] {
				self[i] = other[i];
			}
		}
		self
	}
	pub fn min(mut self, other: Vector<N, T>) -> Vector<N, T>
	where
		T: PartialOrd + Copy,
	{
		for i in Dimensions(0..N) {
			if other[i] < self[i] {
				self[i] = other[i];
			}
		}
		self
	}

	pub fn distance(self, other: Vector<N, T>) -> T
	where
		T: Copy + Zero + Sqrt,
		T: Add<T, Output = T>,
		T: Sub<T, Output = T>,
		T: Mul<T, Output = T>,
	{
		(self - other).length()
	}
}

impl<const N: usize, T> From<[T; N]> for Vector<N, T> {
	fn from(value: [T; N]) -> Self {
		Self(value)
	}
}

impl<const N: usize, T: std::fmt::Display> std::fmt::Display for Vector<N, T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "[")?;
		if N > 0 {
			write!(f, "{}", self[X])?;
		}
		for i in Dimensions(1..N) {
			write!(f, ", {}", self[i])?;
		}
		write!(f, "]")?;
		Ok(())
	}
}

impl<T: Mul<Output = T> + Sub<Output = T>> Vector<3, T> {
	pub fn cross(self, other: Self) -> Self
	where
		T: Copy,
	{
		Self::from([
			self[Y] * other[Z] - self[Z] * other[Y],
			self[Z] * other[X] - self[X] * other[Z],
			self[X] * other[Y] - self[Y] * other[X],
		])
	}
}

impl<const N: usize, T> std::convert::AsRef<[T; N]> for Vector<N, T> {
	fn as_ref(&self) -> &[T; N] {
		&self.0
	}
}

impl<const N: usize, T> std::convert::AsMut<[T; N]> for Vector<N, T> {
	fn as_mut(&mut self) -> &mut [T; N] {
		&mut self.0
	}
}

impl<const N: usize, T> std::convert::AsRef<[T]> for Vector<N, T> {
	fn as_ref(&self) -> &[T] {
		&self.0
	}
}

impl<const N: usize, T> std::convert::AsMut<[T]> for Vector<N, T> {
	fn as_mut(&mut self) -> &mut [T] {
		&mut self.0
	}
}

impl<const N: usize, T> Serialize for Vector<N, T>
where
	[T; N]: Serialize,
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.0.serialize(serializer)
	}
}

impl<'de, const N: usize, T> Deserialize<'de> for Vector<N, T>
where
	[T; N]: Deserialize<'de>,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Ok(Self(<[T; N]>::deserialize(deserializer)?))
	}
}
