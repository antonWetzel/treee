use std::hash::Hash;

use math::{ Vector, X, Y, Z };


#[derive(Debug)]
pub struct QuadTree<T>
where
	T: Eq + Copy + Hash,
{
	root: Node<T>,
	min: Vector<2, usize>,
	size: usize,
}


impl<T: Eq + Copy + Hash> QuadTree<T> {
	pub fn new(min: Vector<2, usize>, size: usize) -> Self {
		Self { root: Node::Leaf(None), min, size }
	}


	pub fn set(&mut self, position: Vector<3, usize>, value: T) {
		self.root.set(position, value, self.min, self.size);
	}


	pub fn get(&self, position: Vector<3, usize>, max_distance: usize) -> Option<T> {
		self.root
			.get_nearest(position, self.min, self.size, max_distance)
			.map(|(value, _)| value)
	}
}


#[derive(Debug)]
enum Node<T>
where
	T: Eq + Copy + Hash,
{
	Branch(Box<[Node<T>; 4]>),
	Leaf(Option<(T, usize)>),
}


impl<T: Eq + Copy + Hash> Node<T> {
	pub fn set(&mut self, position: Vector<3, usize>, value: T, mut min: Vector<2, usize>, mut size: usize) {
		match self {
			Node::Branch(children) => {
				let mut idx = 0;
				size /= 2;
				if position[X] >= min[X] + size {
					idx += 1;
					min[X] += size;
				}
				if position[Z] >= min[Y] + size {
					idx += 2;
					min[Y] += size;
				}
				children[idx].set(position, value, min, size)
			},
			Node::Leaf(old) => {
				if size == 1 {
					*old = Some((value, position[Y]));
					return;
				}
				let mut idx = 0;
				size /= 2;
				if position[X] >= min[X] + size {
					idx += 1;
					min[X] += size;
				}
				if position[Z] >= min[Y] + size {
					idx += 2;
					min[Y] += size;
				}

				let mut children = [
					Node::Leaf(None),
					Node::Leaf(None),
					Node::Leaf(None),
					Node::Leaf(None),
				];
				children[idx].set(position, value, min, size);
				*self = Node::Branch(Box::new(children));
			},
		}
	}


	pub fn get_nearest(
		&self,
		position: Vector<3, usize>,
		min: Vector<2, usize>,
		mut size: usize,
		max: usize,
	) -> Option<(T, usize)> {
		match self {
			Node::Branch(children) => {
				size /= 2;
				let x = if position[X] < min[X] + size {
					[0, 1]
				} else {
					[1, 0]
				};
				let z = if position[Z] < min[Y] + size {
					[0, 1]
				} else {
					[1, 0]
				};
				let mut best = None;
				let mut dist = max;
				for x in x {
					for z in z {
						let min = min + Vector::new([x, z]) * size;
						let diff = min[X].saturating_sub(position[X])
							+ position[X].saturating_sub(min[X] + size)
							+ min[Y].saturating_sub(position[Z])
							+ position[Z].saturating_sub(min[Y] + size);
						if diff >= dist {
							continue;
						}
						let Some((value, score)) = children[x + z * 2].get_nearest(position, min, size, max) else {
							continue;
						};
						if score >= dist {
							continue;
						}
						dist = score;
						best = Some((value, score))
					}
				}
				best
			},
			&Node::Leaf(Some((value, y))) => Some((
				value,
				y.abs_diff(position[Y]) + min[X].abs_diff(position[X]) + min[Y].abs_diff(position[Z]),
			)),
			Node::Leaf(None) => None,
		}
	}
}
