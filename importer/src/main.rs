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

fn main() {
	let mut args = std::env::args().rev().collect::<Vec<_>>();
	args.pop();
	let input = args.pop().unwrap();
	let output = args.pop().unwrap();

	let mut reader = las::Reader::from_path(&input).expect("Unable to open reader");

	let header_min = reader.header().bounds().min;
	let header_max = reader.header().bounds().max;
	let min = Vector::new([header_min.x, header_min.z, -header_max.y]);
	let max = Vector::new([header_max.x, header_max.z, -header_min.y]);
	let diff = max - min;
	let pos = min + diff / 2.0;

	let mut writer = Writer::new(output);
	let mut tree = Tree::new(
		&mut writer,
		(min - pos).map(|v| v as f32),
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
		let res = DataPoint {
			position: [
				(point.x - pos[X]) as f32,
				(point.z - pos[Y]) as f32,
				(-point.y - pos[Z]) as f32,
			]
			.into(),
			color: if let Some(color) = point.color {
				[
					color.red as f32 / u16::MAX as f32,
					color.green as f32 / u16::MAX as f32,
					color.blue as f32 / u16::MAX as f32,
				]
				.into()
			} else {
				let b = (point.z - min[Y]) / diff[Y];
				let b = b * (GRADIENT.len() - 1) as f64;
				let idx = (b as usize).clamp(0, GRADIENT.len() - 2);
				let frac = (b - idx as f64).clamp(0.0, 1.0) as f32;
				GRADIENT[idx] * (1.0 - frac) + GRADIENT[idx + 1] * frac
			},
		};
		tree.insert(res, &mut writer);
		progress.increase();
	}

	let (tree, project) = tree.flatten();
	writer.save_project(&project);
	tree.calculate_properties(&mut writer, project);
}
