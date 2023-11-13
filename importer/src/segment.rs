use math::{Vector, X, Y, Z};

use crate::cache::{Cache, CacheEntry, CacheIndex};

pub struct Segment {
	data: CacheEntry<Vector<3, f32>>,
}

impl Segment {
	pub fn points(self) -> Vec<Vector<3, f32>> {
		self.data.read()
	}
}

enum Node {
	Branch(Box<[Option<Node>; 4]>),
	Leaf(CacheIndex),
}

impl Node {
	fn result(self, res: &mut Vec<Segment>, cache: &mut Cache<Vector<3, f32>>) {
		match self {
			Node::Branch(children) => {
				for child in children.into_iter().flatten() {
					child.result(res, cache)
				}
			},
			Node::Leaf(index) => res.push(Segment { data: cache.read(index) }),
		}
	}
}

/// ### Idea
/// 1. split into quadtree
/// 	- dynamic or fixed physical size
/// 1. identify ground level
/// 1. slice above ground
/// 1. identify trunks
/// 1. grow trees from identified segments
pub struct Segmenter {
	root: Node,
	cache: Cache<Vector<3, f32>>,
	min: Vector<2, f32>,
	size: f32,
}

impl Segmenter {
	pub fn new(min: Vector<3, f32>, max: Vector<3, f32>) -> Self {
		let mut cache = Cache::new(64);
		let entry = cache.new_entry();
		Self {
			cache,
			min: [min[X], min[Z]].into(),
			size: (max[X] - min[X]).max(max[Z] - min[Z]),

			// root: Node::Branch(Box::new([None, None, None, None])),
			root: Node::Leaf(entry),
		}
	}

	pub fn add_point(&mut self, point: Vector<3, f32>) {
		let mut size = self.size;
		let mut node = &mut self.root;
		let mut pos = self.min;
		loop {
			node = match node {
				Node::Branch(children) => {
					size /= 2.0;
					let idx = if point[X] >= pos[X] + size {
						pos[X] += size;
						1
					} else {
						0
					} + if point[Z] >= pos[Y] + size {
						pos[Y] += size;
						2
					} else {
						0
					};

					if children[idx].is_none() {
						children[idx] = Some(if size > 200000000000.0 {
							Node::Branch(Box::new([None, None, None, None]))
						} else {
							Node::Leaf(self.cache.new_entry())
						});
					};

					children[idx].as_mut().unwrap()
				},
				Node::Leaf(index) => {
					self.cache.add_point(index, point);
					break;
				},
			};
		}
	}

	pub fn result(mut self) -> Vec<Segment> {
		let mut res = Vec::new();
		self.root.result(&mut res, &mut self.cache);
		res
	}
}
