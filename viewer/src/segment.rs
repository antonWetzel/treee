use std::{
	io::{BufWriter, Read, Write},
	num::NonZeroU32,
	path::{Path, PathBuf},
	sync::mpsc::TryRecvError,
};

use math::{Vector, X, Y, Z};

use crate::state::State;

pub enum MeshState {
	None,
	Progress(
		Vec<u32>,
		render::Mesh,
		std::sync::mpsc::Receiver<Option<Vector<3, usize>>>,
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
	path: PathBuf,
	point_cloud: render::PointCloud,
	property: render::PointCloudProperty,
	pub mesh: MeshState,
	pub render: MeshRender,
	index: NonZeroU32,
	points: Vec<render::Point>,
	pub alpha: f32,
	pub sub_sample_distance: f32,

	pub information: common::Segment,
}

impl Segment {
	pub fn new(state: &State, mut path: PathBuf, property: &str, index: NonZeroU32) -> Self {
		path.push(format!("{}/points.data", index));
		let mut file = std::fs::OpenOptions::new().read(true).open(&path).unwrap();
		let length = file.metadata().unwrap().len();
		let mut points = bytemuck::zeroed_vec::<render::Point>(length as usize / std::mem::size_of::<render::Point>());
		file.read_exact(bytemuck::cast_slice_mut(&mut points))
			.unwrap();

		let point_cloud = render::PointCloud::new(state, &points);
		// path.set_file_name("mesh.data");
		// let mesh = (|| {
		// 	let mut file = File::open(&path).ok()?;
		// 	let size = file.metadata().map(|m| m.len() as usize).ok()?;
		// 	let mut data = bytemuck::zeroed_vec::<u32>(size / std::mem::size_of::<u32>());
		// 	file.read_exact(bytemuck::cast_slice_mut(&mut data)).ok()?;
		// 	let mesh = render::Mesh::new(state, &data);
		// 	Some(mesh)
		// })();

		path.set_file_name("segment.information");
		let information = common::Segment::load(&path);

		path.set_file_name(format!("{}.data", property));
		Self {
			property: Self::load_property(state, &path),
			point_cloud,
			path,
			mesh: MeshState::None,
			render: MeshRender::Points,
			index,
			information,
			points,
			alpha: 0.5,
			sub_sample_distance: 0.1,
		}
	}

	pub fn change_property(&mut self, state: &State, property: &str) {
		self.path.set_file_name(format!("{}.data", property));
		self.property = Self::load_property(state, &self.path);
	}

	fn load_property(state: &State, path: &Path) -> render::PointCloudProperty {
		let mut file = std::fs::OpenOptions::new().read(true).open(path).unwrap();
		let length = file.metadata().unwrap().len();
		let mut data = bytemuck::zeroed_vec::<u32>(length as usize / std::mem::size_of::<u32>());
		file.read_exact(bytemuck::cast_slice_mut(&mut data))
			.unwrap();
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
							res.push(triangle[X] as u32);
							res.push(triangle[Y] as u32);
							res.push(triangle[Z] as u32);
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
					point.position[X],
					-point.position[Z],
					point.position[Y],
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
