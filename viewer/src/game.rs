use std::{ path::PathBuf, ops::Not };

use common::Project;
use math::{ Vector, X, Y };

use crate::{
	lod,
	segment::Segment,
	state::State,
	tree::{ Tree, LookupName },
	camera,
};


pub struct World {
	window: render::Window,
	game: Game,
	egui: render::egui::Context,
}


impl std::ops::Deref for World {
	type Target = Game;


	fn deref(&self) -> &Self::Target {
		&self.game
	}
}


impl std::ops::DerefMut for World {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.game
	}
}


pub struct Game {
	tree: Tree,
	project: Project,
	path: PathBuf,
	project_time: std::time::SystemTime,

	state: &'static State,
	mouse: input::Mouse,
	mouse_start: Option<Vector<2, f32>>,

	keyboard: input::Keyboard,
	time: Time,
	paused: bool,

	property_options: bool,
	visual_options: bool,
	level_of_detail_options: bool,
	camera_options: bool,
	quit: bool,

	render_time: f32,
}


impl World {
	pub fn new(state: &'static State, path: PathBuf, runner: &render::Runner) -> Self {
		let project = Project::from_file(&path);

		let egui = render::egui::Context::default();
		let window = render::Window::new(state, &runner.event_loop, &project.name, &egui);

		window.set_window_icon(include_bytes!("../assets/png/tree-fill-big.png"));

		#[cfg(windows)] window.set_taskbar_icon(include_bytes!("../assets/png/tree-fill-small.png"));

		let tree = Tree::new(
			state,
			&project,
			path.parent().unwrap().to_owned(),
			project.properties[0].clone(),
			&window,
		);

		Self {
			window,
			egui,
			game: Game {
				paused: false,

				tree,
				project,
				project_time: std::fs::metadata(&path).unwrap().modified().unwrap(),
				path,

				state,
				mouse: input::Mouse::new(),
				mouse_start: None,
				keyboard: input::Keyboard::new(),
				time: Time::new(),

				property_options: false,
				level_of_detail_options: false,
				camera_options: false,
				visual_options: false,
				quit: false,

				render_time: 0.01,
			},
		}
	}


	fn request_redraw(&mut self) {
		self.window.request_redraw();
	}
}


impl Game {
	fn change_project(&mut self, window: &render::Window) {
		let Some(path) = rfd::FileDialog::new()
			.add_filter("Project File", &["epc"])
			.pick_file()
		else {
			return;
		};
		self.path = path;
		self.reload(self.current_poject_time(), window);
	}


	fn check_reload(&mut self, window: &render::Window) {
		let project_time = self.current_poject_time();
		if self.project_time == project_time {
			return;
		}
		if project_time.elapsed().unwrap() < std::time::Duration::from_millis(1000) {
			return;
		}
		self.reload(project_time, window);
	}


	fn current_poject_time(&self) -> std::time::SystemTime {
		self.path
			.metadata()
			.map(|meta| meta.modified().unwrap_or(self.project_time))
			.unwrap_or(self.project_time)
	}


	fn reload(&mut self, project_time: std::time::SystemTime, window: &render::Window) {
		self.project_time = project_time;
		self.project = Project::from_file(&self.path);
		self.tree = Tree::new(
			self.state,
			&self.project,
			self.path.parent().unwrap().to_owned(),
			self.project.properties[0].clone(),
			window,
		);
		window.set_title(&self.project.name);
		window.request_redraw();
	}


	fn raycast(&mut self, window: &render::Window) {
		if self.tree.segment.is_some() {
			return;
		}
		let start = self
			.tree
			.camera
			.ray_origin(self.mouse.position(), window.get_size());
		let direction = self
			.tree
			.camera
			.ray_direction(self.mouse.position(), window.get_size());
		let mut segment_path = self.path.parent().unwrap().to_path_buf();
		segment_path.push("segments");
		if let Some(segment) = self.tree.raycast(start, direction, &segment_path) {
			self.tree.segment = Some(Segment::new(
				self.state,
				segment_path,
				&self.tree.property.0,
				segment,
			));
			window.request_redraw();
		}
	}
}


impl Game {
	fn ui(&mut self, ctx: &render::egui::Context, window: &mut render::Window) {
		const HEIGHT: f32 = 10.0;
		const LEFT: f32 = 100.0;
		const RIGHT: f32 = 150.0;


		use render::egui::*;


		let full = Layout::top_down_justified(Align::Center);
		SidePanel::left("left").resizable(false).default_width(275.0).show(ctx, |ui| {
			if ui.add_sized([ui.available_width(), HEIGHT], Button::new("Load Project")).clicked() {
				self.change_project(window);
			};
			ui.separator();

			if ui.add_sized([ui.available_width(), HEIGHT], SelectableLabel::new(self.property_options, "Property")).clicked() {
				self.property_options = self.property_options.not();
			};
			if self.property_options {
				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Selected"));
					ComboBox::from_id_source("property_selected")
						.width(RIGHT)
						.selected_text(&self.tree.property.1)
						.show_ui(ui, |ui| {
							let mut changed = false;
							for prop in &self.project.properties {
								changed |= ui.selectable_value(&mut self.tree.property.0, prop.0.clone(), &prop.1).changed();
							}
							if changed {
								for prop in &self.project.properties {
									if prop.0 == self.tree.property.0 {
										self.tree.property.1 = prop.1.clone();
									}
								}
								self.tree.loaded_manager.change_property(&self.tree.property.0);
								self.tree.update_lookup(self.state);
								if let Some(seg) = &mut self.tree.segment {
									seg.change_property(self.state, &self.tree.property.0);
								}
							}
						});
				});
			}
			ui.separator();

			let mut seg = self.tree.segment.is_some();
			ui.with_layout(full, |ui| {
				ui.set_enabled(seg);
				if ui.toggle_value(&mut seg, "Segment").changed() && seg.not() {
					self.tree.segment = None;
				}
			});
			if let Some(seg) = &mut self.tree.segment {
				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("ID"));
					ui.add_sized([RIGHT, HEIGHT], Label::new(format!("{}", seg.index())));
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Display"));
					if ui.add_sized([ui.available_width() / 2.0, HEIGHT], SelectableLabel::new(seg.render_mesh.not(), "Points")).clicked() {
						seg.render_mesh = false;
					}
					if ui.add_sized([ui.available_width(), HEIGHT], SelectableLabel::new(seg.render_mesh, "Mesh")).clicked() {
						seg.render_mesh = true;
					}
				});

				for info in &seg.information.values {
					ui.horizontal(|ui| {
						ui.add_sized([LEFT, HEIGHT], Label::new(&info.0));
						ui.add_sized([LEFT, HEIGHT], Label::new(format!("{}", info.1)));
					});
				}
			}
			ui.separator();

			if ui.add_sized([ui.available_width(), HEIGHT], SelectableLabel::new(self.visual_options, "Visual")).clicked() {
				self.visual_options = self.visual_options.not();
			}
			if self.visual_options {
				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Point Size"));
					if ui.add_sized([RIGHT, HEIGHT], Slider::new(&mut self.tree.environment.scale, 0.0 ..= 2.0)).changed() {
						self.tree.environment = render::PointCloudEnvironment::new(
							self.state,
							self.tree.environment.min,
							self.tree.environment.max,
							self.tree.environment.scale,
						);
						window.request_redraw();
					}
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Min"));
					let mut min = self.tree.environment.min as f32 / u32::MAX as f32;
					if ui.add_sized([RIGHT, HEIGHT], Slider::new(&mut min, 0.0 ..= 1.0)).changed() {
						self.tree.environment.min = (min * u32::MAX as f32) as u32;
						self.tree.environment.max = self.tree.environment.max.max(self.tree.environment.min);
						self.tree.environment = render::PointCloudEnvironment::new(
							self.state,
							self.tree.environment.min,
							self.tree.environment.max,
							self.tree.environment.scale,
						);
						window.request_redraw();
					}
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Max"));
					let mut max = self.tree.environment.max as f32 / u32::MAX as f32;
					if ui.add_sized([RIGHT, HEIGHT], Slider::new(&mut max, 0.0 ..= 1.0)).changed() {
						self.tree.environment.max = (max * u32::MAX as f32) as u32;
						self.tree.environment.min = self.tree.environment.min.min(self.tree.environment.max);
						self.tree.environment = render::PointCloudEnvironment::new(
							self.state,
							self.tree.environment.min,
							self.tree.environment.max,
							self.tree.environment.scale,
						);
						window.request_redraw();
					}
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Color Palette"));
					ComboBox::from_id_source("color_palette")
						.selected_text(format!("{:?}", self.tree.lookup_name))
						.width(RIGHT)
						.show_ui(ui, |ui| {
							let mut changed = false;
							changed |= ui.selectable_value(&mut self.tree.lookup_name, LookupName::Warm, "Warm").changed();
							changed |= ui.selectable_value(&mut self.tree.lookup_name, LookupName::Cold, "Cold").changed();
							// changed |= ui.selectable_value(&mut self.tree.lookup_name, LookupName::Wood, "Wood").changed();
							if changed {
								self.tree.update_lookup(self.state);
							}
						});
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Background"));
					ui.style_mut().spacing.interact_size.x = ui.available_width();
					if ui.color_edit_button_rgb(self.tree.background.data_mut()).changed() {
						window.request_redraw();
					}
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Screenshot"));
					if ui.add_sized([RIGHT, HEIGHT], Button::new("Save").min_size(ui.available_size())).clicked() {
						let Some(path) = rfd::FileDialog::new()
							.add_filter("PNG", &["png"])
							.save_file()
						else {
							return;
						};
						window.screen_shot(self.state, &mut self.tree, path);
						window.request_redraw()
					}
				});
			}
			ui.separator();

			ui.with_layout(full, |ui| {
				ui.toggle_value(&mut self.tree.eye_dome_active, "Eye Dome")
			});
			if self.tree.eye_dome_active {
				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Strength"));
					if ui.add_sized([RIGHT, HEIGHT], Slider::new(&mut self.tree.eye_dome.strength, 0.0 ..= 1.0)).changed() {
						self.tree.eye_dome.update_settings(self.state);
					}
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Color"));
					ui.style_mut().spacing.interact_size.x = ui.available_size().x;
					if ui.color_edit_button_rgb(self.tree.eye_dome.color.data_mut()).changed() {
						self.tree.eye_dome.update_settings(self.state);
					}
				});
			};
			ui.separator();

			if ui.add_sized([ui.available_width(), HEIGHT], SelectableLabel::new(self.level_of_detail_options, "Level of Detail")).clicked() {
				self.level_of_detail_options = self.level_of_detail_options.not();
			}
			if self.level_of_detail_options {
				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Mode"));
					ComboBox::from_id_source("level_of_detail_mode")
						.width(RIGHT)
						.selected_text(match self.tree.camera.lod {
							lod::Mode::Auto { .. } => "Automatic",
							lod::Mode::Normal { .. } => "Normal",
							lod::Mode::Level { .. } => "Level",
						})
						.show_ui(ui, |ui| {
							ui.selectable_value(&mut self.tree.camera.lod, lod::Mode::new_auto(
								self.project.depth as usize,
							), "Automatic");
							ui.selectable_value(&mut self.tree.camera.lod, lod::Mode::new_normal(
								self.project.depth as usize,
							), "Normal");
							ui.selectable_value(&mut self.tree.camera.lod, lod::Mode::new_level(
								self.project.depth as usize,
							), "Level");
						});
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Precision"));

					match &mut self.tree.camera.lod {
						lod::Mode::Auto { threshold } => {
							ui.set_enabled(false);
							ui.add_sized([RIGHT, HEIGHT], Slider::new(threshold, 0.0 ..= 10.0));
						}
						lod::Mode::Normal { threshold } => _ = ui.add_sized([RIGHT, HEIGHT], Slider::new(threshold, 0.0 ..= 10.0)),
						lod::Mode::Level { target, max } => _ = ui.add_sized([RIGHT, HEIGHT], Slider::new(target, 0 ..= *max)),
					};
				});
			}
			ui.separator();

			if ui.add_sized([ui.available_width(), HEIGHT], SelectableLabel::new(self.camera_options, "Camera")).clicked() {
				self.camera_options = self.camera_options.not();
			};
			if self.camera_options {
				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Controller"));
					ComboBox::from_id_source("camera_controller")
						.width(RIGHT)
						.selected_text(match self.tree.camera.controller {
							camera::Controller::Orbital { .. } => "Orbital",
							camera::Controller::FirstPerson { .. } => "First Person",
						})
						.show_ui(ui, |ui| {
							let c = self.tree.camera.orbital();
							ui.selectable_value(&mut self.tree.camera.controller, c, "Orbital");
							let c = self.tree.camera.first_person();
							ui.selectable_value(&mut self.tree.camera.controller, c, "First Person");
						});
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new(match self.tree.camera.controller {
						camera::Controller::Orbital { .. } => "Distance",
						camera::Controller::FirstPerson { .. } => "Speed",
					}));
					match &mut self.tree.camera.controller {
						camera::Controller::Orbital { offset } => {
							let old = *offset;
							ui.add_sized([RIGHT, HEIGHT], DragValue::new(offset));
							if *offset < 0.1 {
								*offset = 0.1;
							}
							let diff = *offset - old;
							if diff.abs() > 0.001 {
								self.tree.camera.move_in_view_direction(diff, self.state);
							}
						},
						camera::Controller::FirstPerson { sensitivity } => {
							ui.add_sized([RIGHT, HEIGHT], DragValue::new(sensitivity));
							if *sensitivity < 0.01 {
								*sensitivity = 0.01;
							}
						}
					};
				});

				ui.horizontal_top(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Position"));
					if ui.add_sized([ui.available_width() / 2.0, HEIGHT], Button::new("Save")).clicked() {
						self.tree.camera.save();
					}
					if ui.add_sized([ui.available_width(), HEIGHT], Button::new("Load")).clicked() {
						self.tree.camera.load(self.state);
						window.request_redraw();
					}
				});
			}
		});
	}
}


impl render::Entry for World {
	fn raw_event(&mut self, event: &render::Event) -> bool {
		let response = self.window.window_event(event);
		if response.repaint {
			self.request_redraw();
		}
		response.consumed
	}


	fn render(&mut self, _window_id: render::WindowId) {
		if self.paused {
			return;
		}
		if self.tree.segment.is_none() {
			self.tree.update();
		}

		let raw_input = self.window.egui_winit.take_egui_input(&self.window.window);
		let full_output = self.egui.run(raw_input, |ctx| self.game.ui(ctx, &mut self.window));

		if let Some(time) = self.window.render(self.game.state, &mut self.game.tree, full_output, &self.egui) {
			self.render_time = time;
		}
	}


	fn resize_window(&mut self, _window_id: render::WindowId, size: Vector<2, u32>) {
		self.paused = size[X] == 0 || size[Y] == 0;
		if self.paused {
			return;
		}
		self.window.resized(self.state);
		self.game.tree.camera.cam.set_aspect(self.window.get_aspect());
		self.tree.camera.gpu = render::Camera3DGPU::new(
			self.state,
			&self.tree.camera.cam,
			&self.tree.camera.transform,
		);
		self.request_redraw();
		self.game.tree.eye_dome.update_depth(self.game.state, self.window.depth_texture());
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
			self.game.tree.camera.movement(direction, self.state);
			self.request_redraw();
		}

		if self.tree.loaded_manager.update()
			|| (self.tree.loaded_manager.loaded() > 0
				&& self.tree.segment.is_none()
				&& self.game.tree.camera.time(self.render_time))
		{
			self.window.request_redraw();
		}

		self.game.check_reload(&self.window);
	}


	fn key_changed(&mut self, _window_id: render::WindowId, key: input::KeyCode, key_state: input::State) {
		self.keyboard.update(key, key_state);
	}


	fn modifiers_changed(&mut self, modifiers: input::Modifiers) {
		self.keyboard.update_modifiers(modifiers);
	}


	fn mouse_wheel(&mut self, delta: f32) {
		self.game.tree.camera.scroll(delta, self.state);
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
				self.mouse_start = Some(self.mouse.position());
			},
			(input::MouseButton::Left, input::State::Released) => {
				if let Some(start) = self.mouse_start {
					let dist = (start - self.mouse.position()).length();
					if dist < 2.0 {
						self.game.raycast(&self.window);
					}
				}
			},
			_ => { },
		}
	}


	fn mouse_moved(&mut self, _window_id: render::WindowId, position: Vector<2, f32>) {
		let delta = self.mouse.delta(position);
		if self.mouse.pressed(input::MouseButton::Left) {
			self.game.tree.camera.rotate(delta, self.state);
			self.request_redraw();
		}
	}


	fn exit(&self) -> bool {
		self.quit
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
