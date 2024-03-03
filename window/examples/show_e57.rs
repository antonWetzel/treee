use std::sync::Arc;

use math::Vector;
use pollster::FutureExt;
use project::Point;
use render::{PointCloud, PointCloudProperty};
use window::{
	tree::{Scene, TreeContext},
	CustomState, Game, State,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
	env_logger::init();

	let mut runner = render::Runner::new()?;

	async move {
		let egui = render::egui::Context::default();
		let (state, window) = render::State::new("e57 viewer", &runner, &egui).await?;
		let state = Arc::new(State::new(state));
		let tree = window::tree::Tree::new(
			state.clone(),
			("todo: Remove this entire argument".into(), "Too".into(), 42),
			&window,
			ProjectScene::new(state.clone()),
		);
		runner.run(&mut Game::new(window, tree, state, ProjectState {}))?;
		Ok(())
	}
	.block_on()
}

struct ProjectState {}

impl CustomState for ProjectState {
	type Scene = ProjectScene;
}

struct ProjectScene {
	pc: PointCloud,
	property: PointCloudProperty,
}

impl ProjectScene {
	fn new(state: Arc<State>) -> Self {
		let pc = PointCloud::new(
			&state,
			&[Point {
				position: Vector::new([0., 0., 0.]),
				normal: Vector::new([0., 0., 0.]),
				size: 0.09672646,
			}],
		);
		let property = render::PointCloudProperty::new_empty(&state);
		Self { pc, property }
	}
}

impl Scene for ProjectScene {
	fn update(&mut self, _view_checker: window::lod::Checker, _camera: &window::camera::Camera) {
		//called many times
	}

	fn render<'a>(
		&'a self,
		state: &'a State,
		tree: &'a window::tree::Tree<Self>,
		render_pass: &mut render::RenderPass<'a>,
	) {
		render::PointCloudExt::<_, _, TreeContext>::render_point_clouds(
			render_pass,
			self,
			state,
			&tree.context,
			&tree.context.camera.gpu,
			&tree.context.lookup,
			&tree.context.environment,
		);
	}
}

impl<T> render::PointCloudRender<T> for ProjectScene {
	fn render<'a>(&'a self, _context: &'a T, point_cloud_pass: &mut render::PointCloudPass<'a>) {
		self.pc.render(point_cloud_pass, &self.property)
	}
}
