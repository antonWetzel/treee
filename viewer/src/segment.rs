use std::{
	io::Read,
	num::NonZeroU32,
	path::{Path, PathBuf},
};

use crate::state::State;

const MAX_SIZE: usize = 1 << 15;

enum Data<T> {
	Single(T),
	Split(Vec<T>),
}

pub struct Segment {
	path: PathBuf,
	point_cloud: Data<render::PointCloud>,
	property: Data<render::PointCloudProperty>,
}

impl Segment {
	pub fn new(state: &State, mut path: PathBuf, property: &str, index: NonZeroU32) -> Self {
		path.push(format!("{}/points.data", index));
		let mut file = std::fs::OpenOptions::new().read(true).open(&path).unwrap();
		let length = file.metadata().unwrap().len();
		let mut points = bytemuck::zeroed_vec::<render::Point>(length as usize / std::mem::size_of::<render::Point>());
		file.read_exact(bytemuck::cast_slice_mut(&mut points))
			.unwrap();

		let point_cloud = if points.len() <= MAX_SIZE {
			Data::Single(render::PointCloud::new(state, &points))
		} else {
			Data::Split(
				points
					.chunks(MAX_SIZE)
					.map(|data| render::PointCloud::new(state, data))
					.collect(),
			)
		};

		path.set_file_name(format!("{}.data", property));
		Self {
			property: Self::load_property(state, &path),
			point_cloud,
			path,
		}
	}

	pub fn change_property(&mut self, state: &State, property: &str) {
		self.path.set_file_name(format!("{}.data", property));
		self.property = Self::load_property(state, &self.path);
	}

	fn load_property(state: &State, path: &Path) -> Data<render::PointCloudProperty> {
		let mut file = std::fs::OpenOptions::new().read(true).open(path).unwrap();
		let length = file.metadata().unwrap().len();
		let mut data = bytemuck::zeroed_vec::<u32>(length as usize / std::mem::size_of::<u32>());
		file.read_exact(bytemuck::cast_slice_mut(&mut data))
			.unwrap();
		if data.len() <= MAX_SIZE {
			Data::Single(render::PointCloudProperty::new(state, &data))
		} else {
			Data::Split(
				data.chunks(MAX_SIZE)
					.map(|data| render::PointCloudProperty::new(state, data))
					.collect(),
			)
		}
	}
}

impl render::PointCloudRender for Segment {
	fn render<'a>(&'a self, point_cloud_pass: &mut render::PointCloudPass<'a>) {
		match (&self.point_cloud, &self.property) {
			(Data::Single(point_cloud), Data::Single(property)) => point_cloud.render(point_cloud_pass, property),
			(Data::Split(point_clouds), Data::Split(properties)) => {
				for (point_cloud, property) in point_clouds.iter().zip(properties) {
					point_cloud.render(point_cloud_pass, property);
				}
			},
			_ => unreachable!(),
		}
	}
}
