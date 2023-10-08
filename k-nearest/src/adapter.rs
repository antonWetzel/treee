use math::Dimension;

pub trait Adapter<const N: usize, Value, Point>
where
	Value: Copy + Default,
{
	fn get(point: &Point, dimension: Dimension) -> Value;
	fn get_all(point: &Point) -> [Value; N] {
		let mut values = [Value::default(); N];
		values
			.iter_mut()
			.enumerate()
			.for_each(|(d, value)| *value = Self::get(point, Dimension(d)));
		values
	}
}
