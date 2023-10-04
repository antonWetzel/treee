mod calculations;
mod data_point;
mod progress;
mod tree;
mod writer;

use data_point::DataPoint;
use las::Read;
use math::{Vector, X, Y, Z};
use writer::Writer;

use tree::Tree;

use crate::progress::Progress;

const GRADIENT: [Vector<3, f32>; 3] = [
	Vector::new([0.2, 1.0, 1.0]),
	Vector::new([1.0, 1.0, 0.2]),
	Vector::new([1.0, 0.2, 0.2]),
];

fn cast(vec: Vector<3, f64>) -> Vector<3, f32> {
	[vec[X] as f32, vec[Y] as f32, vec[Z] as f32].into()
}

fn flip(vec: Vector<3, f32>) -> Vector<3, f32> {
	[vec[X], vec[Z], vec[Y]].into()
	// todo: negative z component, but fix min corner
}

fn main() {
	let mut args = std::env::args().rev().collect::<Vec<_>>();
	args.pop();
	let input = args.pop().unwrap();
	let output = args.pop().unwrap();

	let mut reader = las::Reader::from_path(&input).expect("Unable to open reader");

	let min = reader.header().bounds().min;
	let min = Vector::new([min.x, min.y, min.z]);
	let max = reader.header().bounds().max;
	let max = Vector::new([max.x, max.y, max.z]);
	let diff = max - min;
	let pos = min + diff / 2.0;
	let mut writer = Writer::new(output);
	let mut tree = Tree::new(
		&mut writer,
		flip(cast(min - pos)),
		diff[X].max(diff[Y]).max(diff[Z]) as f32,
	);

	let (sender, reciever) = crossbeam::channel::bounded(4);
	let mut progress = Progress::new("Import".into(), reader.header().number_of_points() as usize);
	std::thread::spawn(move || {
		for point in reader.points().map(|p| p.expect("Unable to read point")) {
			sender.send(point).unwrap();
		}
	});

	for point in reciever {
		let mut res = DataPoint::default();
		res.position = flip(
			[
				(point.x - pos[X]) as f32,
				(point.y - pos[Y]) as f32,
				(point.z - pos[Z]) as f32,
			]
			.into(),
		);
		res.color = if let Some(color) = point.color {
			[
				color.red as f32 / u16::MAX as f32,
				color.green as f32 / u16::MAX as f32,
				color.blue as f32 / u16::MAX as f32,
			]
			.into()
		} else {
			[1.0, 1.0, 1.0].into()
			// 	let b = (point.z - min[Z]) / (max[Z] - min[Z]);
			// 	let b = b * (GRADIENT.len() - 1) as f64;
			// 	let idx = (b as usize).clamp(0, GRADIENT.len() - 2);
			// 	let frac = (b - idx as f64).clamp(0.0, 1.0) as f32;
			// 	GRADIENT[idx] * (1.0 - frac) + GRADIENT[idx + 1] * frac
		};
		tree.insert(res, &mut writer);
		progress.increase();
	}

	let (tree, mut project) = tree.flatten();
	project.statistics.center = [0.0, 0.0, 0.0].into();
	writer.save_project(&project);
	tree.calculate_properties(&mut writer, project);
}
