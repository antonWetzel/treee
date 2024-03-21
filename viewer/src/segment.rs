use std::{
	io::{BufWriter, Write},
	num::NonZeroU32,
	sync::mpsc::TryRecvError,
};

use nalgebra as na;

use crate::{reader::Reader, state::State};

pub enum MeshState {
	None,
	Progress(
		Vec<u32>,
		render::Mesh,
		std::sync::mpsc::Receiver<Option<na::Point3<usize>>>,
	),
	Done(render::Mesh),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshRender {
	Points,
	Mesh,
	MeshLines,
}

pub struct Segment {
	point_cloud: render::PointCloud,
	property: render::PointCloudProperty,
	pub mesh: MeshState,
	pub render: MeshRender,
	index: NonZeroU32,
	points: Vec<project::Point>,
	pub alpha: f32,
	pub sub_sample_distance: f32,

	pub show_grid: bool,
	pub grid: render::Lines,
}

impl Segment {
	pub fn new(state: &State, reader: &mut Reader, index: NonZeroU32) -> Self {
		let points = reader.get_points(index.get() as usize - 1);

		let mut min_x = points[0].position.x;
		let mut max_x = min_x;
		let mut min_y = points[0].position.y;
		let mut min_z = points[0].position.z;
		let mut max_z = min_z;

		for p in points.iter() {
			if p.position.x < min_x {
				min_x = p.position.x;
			} else if p.position.x > max_x {
				max_x = p.position.x;
			}

			if p.position.y < min_y {
				min_y = p.position.y;
			}

			if p.position.z < min_z {
				min_z = p.position.z;
			} else if p.position.z > max_z {
				max_z = p.position.z;
			}
		}
		let min_x = min_x as isize;
		let max_x = max_x.ceil() as isize;
		let min_z = min_z as isize;
		let max_z = max_z.ceil() as isize;

		let point_cloud = render::PointCloud::new(state, &points);

		let grid = {
			let amount = (max_x - min_x + 1) as usize * 2 + (max_z - min_z + 1) as usize * 2;
			let mut points = Vec::with_capacity(amount);
			let mut indices = Vec::with_capacity(amount);
			for i in min_x..=max_x {
				let off = points.len() as u32;
				points.push(na::Point3::new(i as f32, min_y, min_z as f32));
				points.push(na::Point3::new(i as f32, min_y, max_z as f32));

				indices.push(off + 0);
				indices.push(off + 1);
			}

			for i in min_z..=max_z {
				let off = points.len() as u32;
				points.push(na::Point3::new(min_x as f32, min_y, i as f32));
				points.push(na::Point3::new(max_x as f32, min_y, i as f32));

				indices.push(off + 0);
				indices.push(off + 1);
			}

			render::Lines::new(&state, &points, &indices)
		};

		Self {
			property: Self::load_property(state, reader, index.get() as usize - 1),
			point_cloud,
			mesh: MeshState::None,
			render: MeshRender::Points,
			index,
			points,
			alpha: 0.5,
			sub_sample_distance: 0.1,

			show_grid: false,
			grid,
		}
	}

	pub fn change_property(&mut self, state: &State, reader: &mut Reader) {
		self.property = Self::load_property(state, reader, self.index.get() as usize - 1);
	}

	fn load_property(state: &State, reader: &mut Reader, index: usize) -> render::PointCloudProperty {
		let data = reader.get_property(index);
		render::PointCloudProperty::new(state, &data)
	}

	pub fn index(&self) -> NonZeroU32 {
		self.index
	}

	pub fn update(&mut self, state: &State) {
		let mut update = false;
		match &mut self.mesh {
			MeshState::None | MeshState::Done(_) => {},
			MeshState::Progress(res, _, reciever) => {
				let before = res.len() / 1000;
				loop {
					match reciever.try_recv() {
						Ok(None) => {},
						Ok(Some(triangle)) => {
							res.push(triangle.x as u32);
							res.push(triangle.y as u32);
							res.push(triangle.z as u32);
						},
						Err(TryRecvError::Empty) => break,
						Err(TryRecvError::Disconnected) => {
							let mesh = render::Mesh::new(state, res);
							self.mesh = MeshState::Done(mesh);
							return;
						},
					}
				}
				let after = res.len() / 1000;
				update = before != after;
			},
		}
		if update {
			match &mut self.mesh {
				MeshState::None | MeshState::Done(_) => {},
				MeshState::Progress(res, mesh, _) => {
					*mesh = render::Mesh::new(state, res);
				},
			}
		}
	}

	pub fn triangulate(&mut self, state: &State) {
		let (sender, reciever) = std::sync::mpsc::channel();
		let points = self.points.iter().map(|p| p.position).collect::<Vec<_>>();
		let alpha = self.alpha;
		let sub_sample_distance = self.sub_sample_distance;
		std::thread::spawn(move || {
			_ = triangulation::triangulate(&points, alpha, sub_sample_distance, sender);
		});
		self.mesh = MeshState::Progress(Vec::new(), render::Mesh::new(state, &[]), reciever);
	}

	pub fn save(&self) {
		let Some(location) = rfd::FileDialog::new()
			.add_filter("File", &["ply"])
			.save_file()
		else {
			return;
		};

		let mut min = na::Point {
			coords: na::vector![f32::MAX, f32::MAX, f32::MAX],
		};
		let mut max = na::Point {
			coords: na::vector![f32::MIN, f32::MIN, f32::MIN],
		};
		for &point in &self.points {
			for d in 0..3 {
				min[d] = min[d].min(point.position[d]);
				max[d] = max[d].max(point.position[d]);
			}
		}
		let diff = na::center(&min, &max);

		let mut file = BufWriter::new(std::fs::File::create(location).unwrap());
		file.write_all(b"ply\n").unwrap();
		file.write_all(b"format ascii 1.0\n").unwrap();
		file.write_all(format!("element vertex {}\n", self.points.len()).as_bytes())
			.unwrap();
		file.write_all(b"property float x\n").unwrap();
		file.write_all(b"property float y\n").unwrap();
		file.write_all(b"property float z\n").unwrap();
		// file.write_all(b"property float nx\n").unwrap();
		// file.write_all(b"property float ny\n").unwrap();
		// file.write_all(b"property float nz\n").unwrap();
		// file.write_all(b"property float radius\n").unwrap();
		file.write_all(b"end_header\n").unwrap();
		for &point in &self.points {
			file.write_all(
				format!(
					// "{} {} {} {} {} {} {}\n",
					"{} {} {}\n",
					point.position.x - diff.x,
					-(point.position.z - diff.z),
					point.position.y - min.y,
					// point.normal[X],
					// -point.normal[Z],
					// point.normal[Y],
					// point.size
				)
				.as_bytes(),
			)
			.unwrap();
		}
	}
}

impl render::PointCloudRender for Segment {
	fn render<'a>(&'a self, point_cloud_pass: &mut render::PointCloudPass<'a>) {
		self.point_cloud.render(point_cloud_pass, &self.property);
	}
}

impl render::MeshRender for Segment {
	fn render<'a>(&'a self, mesh_pass: &mut render::MeshPass<'a>) {
		if let MeshState::Done(mesh) | MeshState::Progress(_, mesh, _) = &self.mesh {
			mesh.render(mesh_pass, &self.point_cloud, &self.property);
		}
	}
}
