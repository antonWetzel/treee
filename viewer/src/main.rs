mod camera;
mod loaded_manager;
mod lod;
mod point_cloud;
mod tree;

use common::Project;
pub use render::gpu;

use loaded_manager::LoadedManager;
use math::Vector;
use math::X;
use math::Y;
use pollster::FutureExt;
use tree::Node;
use tree::Tree;

struct FpsCounter {
	count: usize,
	time: f64,
}

impl FpsCounter {
	pub fn new() -> Self {
		Self { count: 0, time: 0.0 }
	}
	pub fn update(&mut self, delta: f64) {
		self.count += 1;
		self.time += delta;
		if self.time >= 1.0 {
			println!("fps: {}", self.count);
			self.count = 0;
			self.time -= 1.0;
		}
	}
}

struct Time {
	last: std::time::Instant,
}

impl Time {
	pub fn new() -> Self {
		Self { last: std::time::Instant::now() }
	}

	pub fn elapsed(&mut self) -> std::time::Duration {
		let delta = self.last.elapsed();
		self.last = std::time::Instant::now();
		delta
	}
}

struct Game {
	window: render::Window,
	tree: Tree,
	pipeline: render::Pipeline3D,
	project: Project,
	fps_counter: FpsCounter,
	path: String,
	project_time: std::time::SystemTime,

	state: &'static render::State,
	mouse: input::Mouse,
	keyboard: input::Keyboard,
	time: Time,
}
impl render::Game for Game {
	fn render(&mut self, _window_id: render::WindowId) {
		let mut checker = lod::Checker::new(&self.tree.camera.lod);
		self.tree.root.update(
			&self.state,
			&mut checker,
			&self.tree.camera,
			&mut self.tree.loaded_manager,
		);
		self.window.render(
			&self.state,
			&self.pipeline,
			&self.tree.camera.gpu,
			&self.tree,
		);
	}

	fn resize_window(&mut self, _window_id: render::WindowId, _size: Vector<2, u32>) -> render::ControlFlow {
		self.window.resized(&self.state);
		self.tree.camera.cam.aspect = self.window.get_aspect();
		self.tree.camera.gpu = gpu::Camera3D::new(
			&self.state,
			&self.tree.camera.cam,
			&self.tree.camera.transform,
		);
		self.camera_changed();
		return render::ControlFlow::Poll;
	}

	fn close_window(&mut self, _window_id: render::WindowId) -> render::ControlFlow {
		return render::ControlFlow::Exit;
	}

	fn time(&mut self) -> render::ControlFlow {
		let delta = self.time.elapsed();
		let mut direction: Vector<2, f32> = [0.0, 0.0].into();
		if self.keyboard.pressed(input::KeyCode::D) {
			direction[X] += 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::S) {
			direction[Y] += 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::A) {
			direction[X] -= 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::W) {
			direction[Y] -= 1.0;
		}
		let l = direction.length();
		if l > 0.0 {
			direction *= 10.0 * delta.as_secs_f32() / l;
			self.tree.camera.movement(direction, &self.state);
			self.camera_changed();
		}
		self.fps_counter.update(delta.as_secs_f64());

		self.check_reload();

		self.window.request_redraw(); // todo: toggle

		self.tree.loaded_manager.update();

		return render::ControlFlow::Poll;
	}

	fn key_changed(
		&mut self,
		_window_id: render::WindowId,
		key: input::KeyCode,
		key_state: input::State,
	) -> render::ControlFlow {
		self.keyboard.update(key, key_state);
		match (key, key_state) {
			(input::KeyCode::Escape, input::State::Pressed) => return render::ControlFlow::Exit,
			(input::KeyCode::R, input::State::Pressed) => {
				self.tree.camera.lod.increase_detail();
				self.window.request_redraw();
			},
			(input::KeyCode::F, input::State::Pressed) => {
				self.tree.camera.lod.decrese_detail();
				self.window.request_redraw();
			},
			(input::KeyCode::L, input::State::Pressed) => {
				self.tree.camera.change_lod(self.project.level as usize);
				self.window.request_redraw();
			},
			(input::KeyCode::C, input::State::Pressed) => {
				self.tree.camera.change_controller();
				self.window.request_redraw();
			},
			(input::KeyCode::N, input::State::Pressed) => {
				self.tree.camera.change_controller();
				self.window.request_redraw();
			},
			_ => {},
		}
		render::ControlFlow::Poll
	}

	fn modifiers_changed(&mut self, modifiers: input::Modifiers) {
		self.keyboard.update_modifiers(modifiers);
	}

	fn mouse_wheel(&mut self, delta: f32) -> render::ControlFlow {
		self.tree.camera.scroll(delta, &self.state);
		self.camera_changed();
		render::ControlFlow::Poll
	}

	fn mouse_pressed(
		&mut self,
		_window_id: render::WindowId,
		button: input::MouseButton,
		button_state: input::State,
	) -> render::ControlFlow {
		self.mouse.update(button, button_state);
		return render::ControlFlow::Poll;
	}

	fn mouse_moved(&mut self, _window_id: render::WindowId, position: Vector<2, f64>) -> render::ControlFlow {
		let delta = self.mouse.delta(position);
		if self.mouse.pressed(input::MouseButton::Left) {
			self.tree.camera.rotate(delta, &self.state);
			self.camera_changed();
		}
		return render::ControlFlow::Poll;
	}
}

impl Game {
	fn camera_changed(&mut self) {
		self.window.request_redraw();
	}

	fn new(state: &'static render::State, path: String, runner: &render::Runner) -> Self {
		let project_path = format!("{}/project.epc", path);
		let project = Project::from_file(&project_path);

		let tree = Tree {
			camera: camera::Camera::new(project.statistics.center, &state),
			root: Node::new(&project.root, &state, &path),
			loaded_manager: LoadedManager::new(&state, path.clone()),
		};

		Self {
			window: render::Window::new(&state, &runner.event_loop, "test"),
			tree,
			pipeline: render::Pipeline3D::new(&state),
			project,
			fps_counter: FpsCounter::new(),
			path: path,
			project_time: std::fs::metadata(project_path).unwrap().modified().unwrap(),

			state: &state,
			mouse: input::Mouse::new(),
			keyboard: input::Keyboard::new(),
			time: Time::new(),
		}
	}

	fn check_reload(&mut self) {
		let project_path = format!("{}/project.epc", self.path);
		let meta = match std::fs::metadata(&project_path) {
			Ok(v) => v,
			Err(_) => return,
		};
		let project_time = match meta.modified() {
			Ok(v) => v,
			Err(_) => return,
		};
		if self.project_time == project_time {
			return;
		}
		if project_time.elapsed().unwrap() < std::time::Duration::from_millis(1000) {
			return;
		}
		self.project_time = project_time;
		self.project = Project::from_file(project_path);
		self.tree.root = Node::new(&self.project.root, &self.state, &self.path);

		self.tree.loaded_manager = LoadedManager::new(&self.state, self.path.clone());
	}
}

fn main() {
	let mut args = std::env::args().rev().collect::<Vec<_>>();
	args.pop();
	let path = args.pop().unwrap();

	let (state, runner) = render::State::new().block_on();
	let state = Box::leak(Box::new(state));

	let mut game = Game::new(state, path, &runner);
	let code = runner.run(&mut game);
	std::process::exit(code);
}
