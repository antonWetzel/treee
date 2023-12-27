use crate::kd_tree::Entry;


pub trait BestSet<Value> {
	fn distance(&self) -> Value;


	fn insert(&mut self, value: Entry<Value>);
}


pub struct FixedSet<'a, Value>
where
	Value: PartialOrd,
{
	values: &'a mut [Entry<Value>],
}


impl<'a, Value> FixedSet<'a, Value>
where
	Value: Copy + PartialOrd,
{
	pub fn new(max_distance: Value, values: &'a mut [Entry<Value>]) -> Self {
		for value in values.iter_mut() {
			*value = Entry {
				distance: max_distance,
				index: usize::MAX,
			};
		}
		Self { values }
	}


	fn fix_down(&mut self, mut index: usize, max_size: usize) {
		loop {
			let mut swap_index = index;
			let mut swap_value = self.values[index].distance;

			let child_left = index * 2 + 1;

			if child_left < max_size && self.values[child_left].distance > swap_value {
				swap_index = child_left;
				swap_value = self.values[child_left].distance;
			}

			let child_right = index * 2 + 2;
			if child_right < max_size && self.values[child_right].distance > swap_value {
				swap_index = child_right;
			}

			if swap_index == index {
				break;
			}
			self.values.swap(index, swap_index);

			index = swap_index;
		}
	}


	pub fn result(mut self) -> usize {
		let mut size = self.values.len();
		for end in (0..size).rev() {
			if self.values[0].index == usize::MAX {
				size = end;
			}
			self.values.swap(0, end);
			self.fix_down(0, end);
		}
		size
	}
}


impl<'a, Value> BestSet<Value> for FixedSet<'a, Value>
where
	Value: Copy + PartialOrd,
{
	fn distance(&self) -> Value {
		self.values[0].distance
	}


	fn insert(&mut self, value: Entry<Value>) {
		// the tree is always full, so we replace the largest element, which may be invalid
		self.values[0] = value;
		self.fix_down(0, self.values.len());
	}
}


pub struct DynamicSet<Value> {
	distance: Value,
	values: Vec<Entry<Value>>,
}


impl<Value: PartialOrd> DynamicSet<Value> {
	pub fn new(distance: Value) -> Self {
		Self { distance, values: Vec::new() }
	}


	pub fn result(self) -> Vec<Entry<Value>> {
		let mut values = self.values;
		values.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
		values
	}
}


impl<Value: Copy> BestSet<Value> for DynamicSet<Value> {
	fn distance(&self) -> Value {
		self.distance
	}


	fn insert(&mut self, value: Entry<Value>) {
		self.values.push(value);
	}
}


pub struct EmptySet<Value>(Option<Value>);


impl<Value> EmptySet<Value> {
	pub fn new(distance: Value) -> Self {
		Self(Some(distance))
	}


	pub fn empty(self) -> bool {
		self.0.is_some()
	}
}


impl<Value: Default + Copy> BestSet<Value> for EmptySet<Value> {
	fn distance(&self) -> Value {
		self.0.unwrap_or_default()
	}


	fn insert(&mut self, _: Entry<Value>) {
		self.0 = None
	}
}
