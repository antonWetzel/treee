use math::{Vector, X, Y, Z};

use crate::point::PointsCollection;

const GRID_SIZE: usize = 64;
const GRID_SIZE_3: usize = GRID_SIZE * GRID_SIZE * GRID_SIZE;
const POINT_SCALE: f32 = 0.95;

#[derive(Clone, Copy)]
struct Cell {
	count: usize,
	position: Vector<3, f32>,
	normal: Vector<3, f32>,
	total_area: f32,

	slice: u32,
	sub_index: u32,
	curve: u32,
}

pub fn grid(children: Vec<PointsCollection>, corner: Vector<3, f32>, size: f32) -> PointsCollection {
	let mut grid = Vec::<Cell>::new();
	grid.resize(
		GRID_SIZE_3,
		Cell {
			count: 0,
			position: Vector::default(),
			normal: Vector::default(),
			total_area: 0.0,

			slice: 0,
			sub_index: 0,
			curve: 0,
		},
	);
	let grid_scale = GRID_SIZE as f32 / size;
	for points in children {
		for (i, point) in points.render.iter().enumerate() {
			let diff = (point.position - corner) * grid_scale;
			let grid_x = (diff[X] as usize).min(GRID_SIZE - 1);
			let grid_y = (diff[Y] as usize).min(GRID_SIZE - 1);
			let grid_z = (diff[Z] as usize).min(GRID_SIZE - 1);

			let grid_pos = grid_x + grid_y * GRID_SIZE + grid_z * GRID_SIZE * GRID_SIZE;

			let cell = &mut grid[grid_pos];

			cell.position += point.position;
			let area = point.size * point.size;
			let weight = area / (cell.total_area + area);
			cell.normal = fast_spherical_linear_interpolation(cell.normal, point.normal, weight);
			cell.total_area += area;
			cell.count += 1;

			cell.slice = points.slice[i];
			cell.sub_index = points.sub_index[i];
			cell.curve = points.curve[i];
		}
	}

	let mut res = PointsCollection::new();
	for cell in grid {
		if cell.count == 0 {
			continue;
		}
		res.add(
			render::Point {
				position: cell.position / cell.count as f32,
				normal: cell.normal,
				size: POINT_SCALE * cell.total_area.sqrt(),
			},
			cell.slice,
			cell.sub_index,
			cell.curve,
		);
	}
	res
}

fn approximate_theta(dist: f32) -> f32 {
	// exact calculation
	//   theta = acos((a*a + b*b + c*c) / (2 * a * b))
	//   theta = acos((1*1 + 1*1 + dist*dist) / (2 * 1 * 1))
	//   theta = acos(1 - dist*dist/2)
	const LINEAR_SCALE: f32 = 0.95;
	const QUADRATIC_SCALE: f32 = 0.1;
	LINEAR_SCALE * dist + QUADRATIC_SCALE * dist * dist
}

fn fast_spherical_linear_interpolation(start: Vector<3, f32>, end: Vector<3, f32>, percent: f32) -> Vector<3, f32> {
	const SAME_DIRECTION_THRESHOLD: f32 = 0.999;
	let overlap = start.dot(end);
	if overlap.abs() >= SAME_DIRECTION_THRESHOLD {
		return start;
	}
	let end_flip = if overlap < 0.0 { -1.0 } else { 1.0 };

	let difference = end * end_flip - start;

	let dist = difference.length();
	let theta = approximate_theta(dist);
	let center_length = (1.0 - dist * dist / (2.0 * 2.0)).sqrt();
	let percent = ((theta * (percent - 0.5).tan()) * center_length / dist) + 0.5;
	let res = start + difference * percent;
	res.normalized()
}
