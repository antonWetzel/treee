use std::path::PathBuf;

use crate::{laz::Laz, octree::Octree, program::World, Error};

#[derive(Debug)]
pub enum Task {
	Load(PathBuf),
	Decompress(Laz),
	// Insert(laz::Chunk),
	Update(usize),
}

#[derive(Debug)]
pub enum TaskResult {
	Error(Error),
	Lookup(render::Lookup),
}

impl Task {
	pub fn run(self, world: &World) -> Result<(), Error> {
		match self {
			Self::Load(path) => {
				let laz = Laz::new(&path)?;
				let chunks = laz.chunks();
				let corner = laz.min;
				let diff = laz.max - laz.min;
				let size = diff.x.max(diff.y).max(diff.z);
				let mut octree = world.octree.write().unwrap();
				let mut point_clouds = world.point_clouds.lock().unwrap();
				*octree = Octree::new(corner, size);
				point_clouds.clear();
				drop(point_clouds);
				drop(octree);

				let lookup = render::Lookup::new_png(
					&world.state,
					include_bytes!("../../viewer/assets/grad_warm.png"),
					(chunks + 1) as u32,
				);
				world.sender.send(TaskResult::Lookup(lookup)).unwrap();

				world.injector.push(Self::Decompress(laz));
			},
			Self::Decompress(mut laz) => {
				let Some(pre_chunk) = laz.get_chunk() else {
					return Ok(());
				};
				world.injector.push(Self::Decompress(laz));
				let idx = pre_chunk.idx;
				let chunk = pre_chunk.decompress();
				for p in chunk {
					let octree = world.octree.read().unwrap();
					octree.insert(p, idx, &world.injector);
				}
			},
			Self::Update(idx) => {
				let octree = world.octree.read().unwrap();
				octree.update(&world.state, &world.point_clouds, idx);
			},
		}
		Ok(())
	}
}
