use std::path::PathBuf;

use crate::{laz::Laz, octree::Octree, program::World, Error};

#[derive(Debug)]
pub enum Task {
	Load(PathBuf),
	Decompress(Laz),
	Update(usize),
	Segment,
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
				let mut octree = world.octree.write().unwrap();
				let laz = Laz::new(&path, octree.generation + 1)?;
				let chunks = laz.chunks();
				let corner = laz.min;
				let diff = laz.max - laz.min;
				let size = diff.x.max(diff.y).max(diff.z);
				let mut point_clouds = world.point_clouds.lock().unwrap();
				*octree = Octree::new(corner, size, octree.generation + 1);
				point_clouds.clear();
				drop(point_clouds);
				drop(octree);

				let lookup = render::Lookup::new_png(
					&world.state,
					include_bytes!("../../viewer/assets/grad_warm.png"),
					(chunks + 1) as u32,
				);
				world.sender.send(TaskResult::Lookup(lookup)).unwrap();
				world.task_sender.send(Self::Decompress(laz)).unwrap();
			},
			Self::Decompress(mut laz) => {
				let generation = laz.generation;
				if generation != world.octree.read().unwrap().generation {
					return Ok(());
				}
				let Some(pre_chunk) = laz.get_chunk() else {
					return Ok(());
				};
				world.task_sender.send(Self::Decompress(laz)).unwrap();
				let idx = pre_chunk.idx;
				let chunk = pre_chunk.decompress();
				let octree = world.octree.read().unwrap();
				if octree.generation != generation {
					return Ok(());
				}
				for p in chunk {
					octree.insert(p, idx, &world.task_sender);
				}
			},
			Self::Update(idx) => {
				let octree = world.octree.read().unwrap();
				octree.update(&world.state, &world.point_clouds, idx);
			},
			Self::Segment => {},
		}
		Ok(())
	}
}
