// helper to create T from constants like PI
pub trait FromF64 {
	fn from_f64(value: f64) -> Self;
}


impl FromF64 for f32 {
	fn from_f64(value: f64) -> f32 {
		value as f32
	}
}


impl FromF64 for f64 {
	fn from_f64(value: f64) -> f64 {
		value
	}
}


// neutral element
pub trait Identity {
	const IDENTITY: Self;
}


impl Identity for f32 {
	const IDENTITY: Self = 1.0;
}


impl Identity for f64 {
	const IDENTITY: Self = 1.0;
}


impl Identity for i32 {
	const IDENTITY: Self = 1;
}


impl Identity for i64 {
	const IDENTITY: Self = 1;
}


impl Identity for u32 {
	const IDENTITY: Self = 1;
}


impl Identity for u64 {
	const IDENTITY: Self = 1;
}


pub trait Zero {
	const ZERO: Self;
}


impl Zero for f32 {
	const ZERO: Self = 0.0;
}


impl Zero for f64 {
	const ZERO: Self = 0.0;
}


impl Zero for i32 {
	const ZERO: Self = 0;
}


impl Zero for i64 {
	const ZERO: Self = 0;
}


impl Zero for isize {
	const ZERO: Self = 0;
}


impl Zero for u32 {
	const ZERO: Self = 0;
}


impl Zero for u64 {
	const ZERO: Self = 0;
}


impl Zero for usize {
	const ZERO: Self = 0;
}


impl<A: Zero, B: Zero> Zero for (A, B) {
	const ZERO: Self = (A::ZERO, B::ZERO);
}


pub trait Trigonometry {
	fn sin(self) -> Self;


	fn cos(self) -> Self;


	fn asin(self) -> Self;


	fn acos(self) -> Self;


	fn tan(self) -> Self;
}


impl Trigonometry for f32 {
	fn sin(self) -> Self {
		self.sin()
	}


	fn cos(self) -> Self {
		self.cos()
	}


	fn asin(self) -> Self {
		self.asin()
	}


	fn acos(self) -> Self {
		self.acos()
	}


	fn tan(self) -> Self {
		self.tan()
	}
}


impl Trigonometry for f64 {
	fn sin(self) -> Self {
		self.sin()
	}


	fn cos(self) -> Self {
		self.cos()
	}


	fn asin(self) -> Self {
		self.asin()
	}


	fn acos(self) -> Self {
		self.acos()
	}


	fn tan(self) -> Self {
		self.tan()
	}
}


pub trait Sqrt {
	fn sqrt(self) -> Self;
}


impl Sqrt for f32 {
	fn sqrt(self) -> Self {
		self.sqrt()
	}
}


impl Sqrt for f64 {
	fn sqrt(self) -> Self {
		self.sqrt()
	}
}


pub trait PowI {
	fn powi(self, n: i32) -> Self;
}


impl PowI for f32 {
	fn powi(self, n: i32) -> Self {
		self.powi(n)
	}
}


impl PowI for f64 {
	fn powi(self, n: i32) -> Self {
		self.powi(n)
	}
}
