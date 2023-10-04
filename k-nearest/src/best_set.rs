pub struct BestSet<'a, Value>
where
	Value: PartialOrd,
{
	values: &'a mut [(Value, usize)],
}

impl<'a, Value> BestSet<'a, Value>
where
	Value: Copy + PartialOrd,
{
	pub fn new(max_distance: Value, values: &'a mut [(Value, usize)]) -> Self {
		for value in values.iter_mut() {
			*value = (max_distance, usize::MAX);
		}
		Self { values }
	}

	pub fn distance(&self) -> Value {
		self.values[0].0
	}

	pub fn insert(&mut self, value: (Value, usize)) {
		self.values[0] = value;
		self.fix_down(0, self.values.len());
	}

	fn fix_down(&mut self, mut index: usize, max_size: usize) {
		loop {
			let mut swap_index = index;
			let mut swap_value = self.values[index].0;

			let child_left = index * 2 + 1;

			if child_left < max_size && self.values[child_left].0 > swap_value {
				swap_index = child_left;
				swap_value = self.values[child_left].0;
			}

			let child_right = index * 2 + 2;
			if child_right < max_size && self.values[child_right].0 > swap_value {
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
			if self.values[0].1 == usize::MAX {
				size = end;
			}
			self.values.swap(0, end);
			self.fix_down(0, end);
		}
		return size;
	}
}
