use std::path::PathBuf;

use common::Project;
use math::{Vector, X, Y};
use render::{ChainExtension, RenderEntry};

use crate::{
	interface::{Interface, InterfaceAction},
	lod,
	state::State,
	tree::Tree,
};

pub struct Game {
	window: render::Window,
	tree: Tree,
	project: Project,
	path: PathBuf,
	project_time: std::time::SystemTime,

	state: &'static State,
	mouse: input::Mouse,
	keyboard: input::Keyboard,
	time: Time,
	always_redraw: bool,
	paused: bool,

	ui: render::UI,
	eye_dome: render::EyeDome,
	interface: Interface,
}

impl Game {
	pub fn new(state: &'static State, path: PathBuf, runner: &render::Runner) -> Self {
		let project = Project::from_file(&path);

		let window = render::Window::new(state, &runner.event_loop, "test");
		let tree = Tree::new(
			state,
			&project,
			path.parent().unwrap().to_owned(),
			window.get_aspect(),
		);

		let eye_dome = render::EyeDome::new(state, window.config(), window.depth_texture(), 5.0, 0.005);
		let ui = render::UI::new(state, window.config());
		let mut interface = Interface::new(state);
		interface.update_eye_dome_settings(eye_dome.strength, eye_dome.sensitivity);

		Self {
			ui,
			eye_dome,
			interface,
			paused: false,

			window,
			tree,
			project,
			project_time: std::fs::metadata(&path).unwrap().modified().unwrap(),
			path,

			always_redraw: false,

			state,
			mouse: input::Mouse::new(),
			keyboard: input::Keyboard::new(),
			time: Time::new(),
		}
	}

	fn request_redraw(&mut self) {
		self.window.request_redraw();
	}

	fn change_project(&mut self) {
		let Some(path) = rfd::FileDialog::new()
			.add_filter("Project File", &["epc"])
			.pick_file()
		else {
			return;
		};
		self.path = path;
		self.reload(self.current_poject_time());
	}

	fn check_reload(&mut self) {
		let project_time = self.current_poject_time();
		if self.project_time == project_time {
			return;
		}
		if project_time.elapsed().unwrap() < std::time::Duration::from_millis(1000) {
			return;
		}
		self.reload(project_time);
	}

	fn current_poject_time(&self) -> std::time::SystemTime {
		std::fs::metadata(&self.path)
			.map(|meta| meta.modified().unwrap_or(self.project_time))
			.unwrap_or(self.project_time)
	}

	fn reload(&mut self, project_time: std::time::SystemTime) {
		self.project_time = project_time;
		self.project = Project::from_file(&self.path);
		self.tree = Tree::new(
			self.state,
			&self.project,
			self.path.parent().unwrap().to_owned(),
			self.window.get_aspect(),
		);
		self.request_redraw();
	}
}

impl RenderEntry for Game {
	fn render(&mut self, _window_id: render::WindowId) {
		if self.paused {
			return;
		}
		let start = std::time::Instant::now();
		self.tree.root.update(
			lod::Checker::new(&self.tree.camera.lod),
			&self.tree.camera,
			&mut self.tree.loaded_manager,
		);

		self.ui.queue(self.state, &self.interface);

		self.window.render(self.state, self);
		let end = std::time::Instant::now();
		self.interface.update_fps(1.0 / (end - start).as_secs_f64());
	}

	fn resize_window(&mut self, _window_id: render::WindowId, size: Vector<2, u32>) -> render::ControlFlow {
		self.paused = size[X] == 0 || size[Y] == 0;
		if self.paused {
			return render::ControlFlow::Wait;
		}
		self.window.resized(self.state);
		self.tree.camera.cam.aspect = self.window.get_aspect();
		self.tree.camera.gpu = render::Camera3DGPU::new(
			self.state,
			&self.tree.camera.cam,
			&self.tree.camera.transform,
		);
		self.request_redraw();
		self.ui.resize(self.state, self.window.config());
		self.eye_dome
			.update_depth(self.state, self.window.depth_texture());

		self.interface.set_scale(self.ui.get_scale());
		render::ControlFlow::Wait
	}

	fn close_window(&mut self, _window_id: render::WindowId) -> render::ControlFlow {
		render::ControlFlow::Exit
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
			self.tree.camera.movement(direction, self.state);
			self.request_redraw();
		}

		{
			let amount = 0.5 * delta.as_secs_f32();
			let mut update = false;
			if self.keyboard.pressed(input::KeyCode::U) {
				self.eye_dome.strength /= 1.0 + amount;
				update = true;
			}
			if self.keyboard.pressed(input::KeyCode::I) {
				self.eye_dome.strength *= 1.0 + amount;
				update = true;
			}
			if self.keyboard.pressed(input::KeyCode::J) {
				self.eye_dome.sensitivity /= 1.0 + amount;
				update = true;
			}
			if self.keyboard.pressed(input::KeyCode::K) {
				self.eye_dome.sensitivity *= 1.0 + amount;
				update = true;
			}
			if update {
				self.eye_dome.update_settings(self.state);
				self.interface
					.update_eye_dome_settings(self.eye_dome.strength, self.eye_dome.sensitivity);
			}
		}
		let workload = self.tree.loaded_manager.update();
		if self.interface.update_workload(workload) || self.always_redraw {
			self.window.request_redraw();
		}

		self.check_reload();

		render::ControlFlow::Wait
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
			_ => {},
		}
		render::ControlFlow::Wait
	}

	fn modifiers_changed(&mut self, modifiers: input::Modifiers) {
		self.keyboard.update_modifiers(modifiers);
	}

	fn mouse_wheel(&mut self, delta: f32) -> render::ControlFlow {
		self.tree.camera.scroll(delta, self.state);
		self.request_redraw();
		render::ControlFlow::Wait
	}

	fn mouse_pressed(
		&mut self,
		_window_id: render::WindowId,
		button: input::MouseButton,
		button_state: input::State,
	) -> render::ControlFlow {
		self.mouse.update(button, button_state);
		if let (input::MouseButton::Left, input::State::Pressed) = (button, button_state) {
			match self
				.interface
				.clicked(self.mouse.position() / self.ui.get_scale())
			{
				InterfaceAction::Nothing => {},
				InterfaceAction::Debug => self.request_redraw(),
				InterfaceAction::Open => self.change_project(),
				InterfaceAction::ColorPalette => {
					self.tree.next_lookup(self.state);
					self.request_redraw();
				},
			}
		}
		render::ControlFlow::Wait
	}

	fn mouse_moved(&mut self, _window_id: render::WindowId, position: Vector<2, f32>) -> render::ControlFlow {
		let delta = self.mouse.delta(position);
		if self.mouse.pressed(input::MouseButton::Left) {
			self.tree.camera.rotate(delta, self.state);
			self.request_redraw();
		}
		render::ControlFlow::Wait
	}
}

impl render::Renderable for Game {
	fn render<'a>(&'a self, render_pass: render::RenderPass<'a>) -> render::RenderPass<'a> {
		render_pass
			.next(|render_pass| render::PointCloudPass::start(render_pass, self.state, &self.tree))
			.next(|point_cloud_pass| self.tree.render(point_cloud_pass))
			.next(|point_cloud_pass| point_cloud_pass.end())
	}

	fn post_process<'a>(&'a self, render_pass: render::RenderPass<'a>) -> render::RenderPass<'a> {
		render_pass
			.next(|render_pass| self.eye_dome.render(render_pass))
			.next(|render_pass| render::UIPass::start(render_pass, &self.ui, self.state))
			.next(|ui_pass| self.interface.render(ui_pass))
			.next(|ui_pass| ui_pass.end())
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