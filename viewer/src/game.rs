use std::path::PathBuf;

use common::Project;
use math::{Vector, X, Y, Z};
use ui::Element;

use crate::{
	interface::{Interface, InterfaceAction},
	lod,
	segment::Segment,
	state::State,
	tree::Tree,
};

const DEFAULT_BACKGROUND: Vector<3, f32> = Vector::new([0.1, 0.2, 0.3]);

pub struct Game {
	window: render::Window,
	tree: Tree,
	segment: Option<Segment>,
	project: Project,
	path: PathBuf,
	project_time: std::time::SystemTime,

	state: &'static State,
	mouse: input::Mouse,
	mouse_start: Option<Vector<2, f32>>,

	keyboard: input::Keyboard,
	time: Time,
	paused: bool,

	ui: render::UI<'static>,
	eye_dome: render::EyeDome,
	eye_dome_active: bool,

	interface: Interface,
	interface_active: bool,

	background: Vector<3, f32>,

	quit: bool,

	render_time: f32,
}

impl Game {
	pub fn new(state: &'static State, path: PathBuf, runner: &render::Runner) -> Self {
		let project = Project::from_file(&path);

		let window = render::Window::new(state, &runner.event_loop, &project.name);

		window.set_window_icon(include_bytes!("../assets/png/tree-fill-big.png"));

		#[cfg(windows)]
		window.set_taskbar_icon(include_bytes!("../assets/png/tree-fill-small.png"));

		let tree = Tree::new(
			state,
			&project,
			path.parent().unwrap().to_owned(),
			window.get_aspect(),
			&project.properties[0],
		);

		let eye_dome = render::EyeDome::new(state, window.config(), window.depth_texture(), 0.002);
		let ui = render::UI::new(
			state,
			window.config(),
			include_bytes!("../assets/Urbanist-Bold.ttf"),
		);
		let interface = Interface::new(state, DEFAULT_BACKGROUND);

		Self {
			ui,
			eye_dome,
			eye_dome_active: true,
			interface,
			interface_active: true,

			paused: false,

			window,
			tree,
			project,
			project_time: std::fs::metadata(&path).unwrap().modified().unwrap(),
			path,
			segment: None,

			state,
			mouse: input::Mouse::new(),
			mouse_start: None,
			keyboard: input::Keyboard::new(),
			time: Time::new(),

			background: DEFAULT_BACKGROUND,

			quit: false,

			render_time: 0.01,
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
		self.path
			.metadata()
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
			&self.project.properties[0],
		);
		self.window.set_title(&self.project.name);
		self.request_redraw();
	}

	fn handle_interface_action(&mut self, action: InterfaceAction) {
		match action {
			InterfaceAction::UpdateInterface => self.request_redraw(),
			InterfaceAction::Open => self.change_project(),

			InterfaceAction::BackgroundReset => {
				self.background = DEFAULT_BACKGROUND;
				self.interface
					.reset_background(self.state, DEFAULT_BACKGROUND);
				self.request_redraw();
			},
			InterfaceAction::BackgroundRed(v) => {
				self.background[X] = v;
				self.request_redraw();
			},
			InterfaceAction::BackgroundGreen(v) => {
				self.background[Y] = v;
				self.request_redraw();
			},
			InterfaceAction::BackgroundBlue(v) => {
				self.background[Z] = v;
				self.request_redraw();
			},

			InterfaceAction::ColorPalette => {
				self.tree.next_lookup(self.state);
				self.request_redraw();
			},
			InterfaceAction::Property => {
				self.tree.next_property(&self.project.properties);
				if let Some(segment) = &mut self.segment {
					segment.change_property(
						self.state,
						self.tree.current_property(&self.project.properties),
					);
				}
				self.request_redraw();
			},
			InterfaceAction::EyeDome => {
				self.eye_dome_active = !self.eye_dome_active;
				self.request_redraw();
			},
			InterfaceAction::Camera => {
				self.tree.camera.change_controller();
				self.window.request_redraw();
			},
			InterfaceAction::LevelOfDetail => {
				self.tree.camera.change_lod(self.project.depth as usize);
				self.window.request_redraw();
			},
			InterfaceAction::LevelOfDetailChange(change) => {
				self.tree.camera.lod.change_detail(change);
				self.window.request_redraw();
			},
			InterfaceAction::EyeDomeStrength(v) => {
				self.eye_dome.strength = if v < 0.1 { 0.1 } else { v }.powi(6);
				self.eye_dome.update_settings(self.state);
				self.window.request_redraw();
			},
			InterfaceAction::SliceUpdate(min, max) => {
				self.tree.environment = render::PointCloudEnvironment::new(
					self.state,
					(min * u32::MAX as f32) as u32,
					(max * u32::MAX as f32) as u32,
					self.tree.environment.scale,
				);
				self.window.request_redraw();
			},

			InterfaceAction::ScaleUpdate(scale) => {
				self.tree.environment = render::PointCloudEnvironment::new(
					self.state,
					self.tree.environment.min,
					self.tree.environment.max,
					scale * 2.0,
				);
				self.window.request_redraw();
			},

			InterfaceAction::EyeDomeRed(v) => {
				self.eye_dome.color[X] = v;
				self.eye_dome.update_settings(self.state);
				self.window.request_redraw();
			},
			InterfaceAction::EyeDomeGreen(v) => {
				self.eye_dome.color[Y] = v;
				self.eye_dome.update_settings(self.state);
				self.window.request_redraw();
			},
			InterfaceAction::EyeDomeBlue(v) => {
				self.eye_dome.color[Z] = v;
				self.eye_dome.update_settings(self.state);
				self.window.request_redraw();
			},

			InterfaceAction::SegmentReset => {
				self.segment = None;
				self.interface.disable_segment_info();
				self.window.request_redraw();
			},
		}
	}

	fn raycast(&mut self) {
		if self.segment.is_some() {
			return;
		}
		let start = self
			.tree
			.camera
			.ray_origin(self.mouse.position(), self.window.get_size());
		let direction = self
			.tree
			.camera
			.ray_direction(self.mouse.position(), self.window.get_size());
		let mut segment_path = self.path.parent().unwrap().to_path_buf();
		segment_path.push("segments");
		if let Some(segment) = self.tree.raycast(start, direction, &segment_path) {
			println!("Switch to Segment '{}'", segment);
			self.segment = Some(Segment::new(
				self.state,
				segment_path,
				self.tree.current_property(&self.project.properties),
				segment,
			));
			self.interface.enable_segment_info(
				&self.project.segment_properties,
				self.project.get_segment_values(segment.get() as usize - 1),
			);
			self.request_redraw();
		}
	}
}

impl render::Entry for Game {
	fn render(&mut self, _window_id: render::WindowId) {
		if self.paused {
			return;
		}
		if self.segment.is_none() {
			self.tree.root.update(
				lod::Checker::new(&self.tree.camera.lod),
				&self.tree.camera,
				&mut self.tree.loaded_manager,
			);
		}

		self.ui.queue(self.state, &self.interface);

		if let Some(time) = self.window.render(self.state, self) {
			self.render_time = time;
		}
	}

	fn resize_window(&mut self, _window_id: render::WindowId, size: Vector<2, u32>) {
		self.paused = size[X] == 0 || size[Y] == 0;
		if self.paused {
			return;
		}
		self.window.resized(self.state);
		self.tree.camera.cam.set_aspect(self.window.get_aspect());
		self.tree.camera.gpu = render::Camera3DGPU::new(
			self.state,
			&self.tree.camera.cam,
			&self.tree.camera.transform,
		);
		self.request_redraw();
		self.ui.resize(self.state, self.window.config());
		self.eye_dome
			.update_depth(self.state, self.window.depth_texture());

		self.interface.resize(
			self.state,
			ui::Rect {
				min: Vector::default(),
				max: self.window.get_size(),
			},
		);
	}

	fn close_window(&mut self, _window_id: render::WindowId) {
		self.quit = true;
	}

	fn time(&mut self) {
		let delta = self.time.elapsed();
		let mut direction: Vector<2, f32> = [0.0, 0.0].into();
		if self.keyboard.pressed(input::KeyCode::KeyD) || self.keyboard.pressed(input::KeyCode::ArrowRight) {
			direction[X] += 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyS) || self.keyboard.pressed(input::KeyCode::ArrowDown) {
			direction[Y] += 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyA) || self.keyboard.pressed(input::KeyCode::ArrowLeft) {
			direction[X] -= 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyW) || self.keyboard.pressed(input::KeyCode::ArrowUp) {
			direction[Y] -= 1.0;
		}
		let l = direction.length();
		if l > 0.0 {
			direction *= 10.0 * delta.as_secs_f32() / l;
			self.tree.camera.movement(direction, self.state);
			self.request_redraw();
		}

		if self.tree.loaded_manager.update()
			|| (self.tree.loaded_manager.loaded() > 0
				&& self.segment.is_none()
				&& self.tree.camera.time(self.render_time))
		{
			self.window.request_redraw();
		}

		self.check_reload();
	}

	fn key_changed(&mut self, _window_id: render::WindowId, key: input::KeyCode, key_state: input::State) {
		self.keyboard.update(key, key_state);

		match (key, key_state) {
			(input::KeyCode::KeyK, input::State::Pressed) => self.tree.camera.save(),
			(input::KeyCode::KeyL, input::State::Pressed) => {
				self.tree.camera.load(self.state);
				self.request_redraw()
			},
			(input::KeyCode::KeyP, input::State::Pressed) => {
				let Some(path) = rfd::FileDialog::new()
					.add_filter("PNG", &["png"])
					.save_file()
				else {
					return;
				};
				let before = self.interface_active;
				self.interface_active = false;
				self.window.screen_shot(self.state, self, path);
				self.interface_active = before;
			},

			_ => {},
		}
	}

	fn modifiers_changed(&mut self, modifiers: input::Modifiers) {
		self.keyboard.update_modifiers(modifiers);
	}

	fn mouse_wheel(&mut self, delta: f32) {
		self.tree.camera.scroll(delta, self.state);
		self.request_redraw();
	}

	fn mouse_button_changed(
		&mut self,
		_window_id: render::WindowId,
		button: input::MouseButton,
		button_state: input::State,
	) {
		self.mouse.update(button, button_state);
		match (button, button_state) {
			(input::MouseButton::Left, input::State::Pressed) => {
				if let Some(action) = self.interface.click(self.state, self.mouse.position()) {
					self.handle_interface_action(action);
					self.mouse_start = None;
				} else {
					self.mouse_start = Some(self.mouse.position());
				}
			},
			(input::MouseButton::Left, input::State::Released) => {
				if let Some(start) = self.mouse_start {
					let dist = (start - self.mouse.position()).length();
					if dist < 2.0 {
						self.raycast();
					}
				}
				if self.interface.release(self.mouse.position()) {
					self.request_redraw();
				}
			},
			_ => {},
		}
	}

	fn mouse_moved(&mut self, _window_id: render::WindowId, position: Vector<2, f32>) {
		let delta = self.mouse.delta(position);
		if let Some(action) = self.interface.hover(
			self.state,
			position,
			self.mouse.pressed(input::MouseButton::Left),
		) {
			self.handle_interface_action(action);
		} else if self.mouse.pressed(input::MouseButton::Left) {
			self.tree.camera.rotate(delta, self.state);
			self.request_redraw();
		}
	}

	fn exit(&self) -> bool {
		self.quit
	}
}

impl render::RenderEntry for Game {
	fn background(&self) -> Vector<3, f32> {
		self.background
	}

	fn render<'a>(&'a self, render_pass: &mut render::RenderPass<'a>) {
		if let Some(segment) = &self.segment {
			// render_pass.render(
			// 	segment,
			// 	(
			// 		self.state,
			// 		&self.tree.camera.gpu,
			// 		&self.tree.lookup,
			// 		&self.tree.environment,
			// 	),
			// );
			render_pass.render(
				segment,
				(self.state, &self.tree.camera.gpu, &self.tree.lookup),
			);
		} else {
			render_pass.render(
				&self.tree,
				(
					self.state,
					&self.tree.camera.gpu,
					&self.tree.lookup,
					&self.tree.environment,
				),
			);
		}
	}

	fn post_process<'a>(&'a self, render_pass: &mut render::RenderPass<'a>) {
		if self.eye_dome_active {
			render_pass.render(&self.eye_dome, ());
		}
		if self.interface_active {
			render_pass.render(&self.interface, (&self.ui, self.state));
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
