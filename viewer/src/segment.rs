use std::{
	io::Read,
	num::NonZeroU32,
	path::{Path, PathBuf},
};

use crate::state::State;

pub struct Segment {
	path: PathBuf,
	point_cloud: render::PointCloud,
	property: render::PointCloudProperty,
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

	fn load_property(state: &State, path: &Path) -> render::PointCloudProperty {
		let mut file = std::fs::OpenOptions::new().read(true).open(path).unwrap();
		let length = file.metadata().unwrap().len();
		let mut data = bytemuck::zeroed_vec::<u32>(length as usize / std::mem::size_of::<u32>());
		file.read_exact(bytemuck::cast_slice_mut(&mut data))
			.unwrap();
		render::PointCloudProperty::new(state, &data)
	}
}

impl render::PointCloudRender for Segment {
	fn render<'a>(&'a self, point_cloud_pass: &mut render::PointCloudPass<'a>) {
		self.point_cloud.render(point_cloud_pass, &self.property);
	}
}
