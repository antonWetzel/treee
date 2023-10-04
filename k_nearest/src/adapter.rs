use math::Dimension;

pub trait Adapter<const N: usize, Value, Point>
where
	Value: Copy + Default,
{
	fn get(point: &Point, dimension: Dimension) -> Value;
	fn get_all(point: &Point) -> [Value; N] {
		let mut values = [Value::default(); N];
		for d in 0..N {
			values[d] = Self::get(point, Dimension(d));
		}
		values
	}
}
