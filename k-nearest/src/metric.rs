pub trait Metric<const N: usize, Value>
where
	Value: PartialOrd,
{
	fn distance(left: &[Value; N], right: &[Value; N]) -> Value;
	fn distance_plane(position: &[Value; N], plane: Value, dimension: usize) -> Value;
}

pub struct EuclideanDistanceSquared;

impl<const N: usize, Value> Metric<N, Value> for EuclideanDistanceSquared
where
	Value: PartialOrd
		+ Default
		+ Copy
		+ std::ops::Mul<Output = Value>
		+ std::ops::Add<Output = Value>
		+ std::ops::Sub<Output = Value>,
{
	fn distance(left: &[Value; N], right: &[Value; N]) -> Value {
		(0..N)
			.map(|d| left[d] - right[d])
			.map(|v| v * v)
			.fold(Value::default(), |sum, v| sum + v)
	}
	fn distance_plane(position: &[Value; N], plane: Value, dimension: usize) -> Value {
		let diff = position[dimension] - plane;
		diff * diff
	}
}
