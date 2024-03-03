use std::{
	ops::{Deref, DerefMut, Not},
	path::PathBuf,
	sync::Arc,
};

use math::{Vector, X, Y};
use project::Project;
use window::State;

use crate::{
	camera, lod,
	reader::Reader,
	segment::{self, MeshRender, Segment},
	tree::{LookupName, Tree},
	Error,
};

pub struct World {
	window: render::Window,
	game: Game<CustomState>,
	egui: render::egui::Context,
}

impl std::ops::Deref for World {
	type Target = Game<CustomState>;

	fn deref(&self) -> &Self::Target {
		&self.game
	}
}

impl std::ops::DerefMut for World {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.game
	}
}

pub struct CustomState {
	project: Project,
	path: Option<PathBuf>,
	project_time: std::time::SystemTime,
}

pub struct Game<TCustomState> {
	tree: Tree,
	custom_state: TCustomState,

	pub state: Arc<State>,
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
}

impl<T> Deref for Game<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.custom_state
	}
}

impl World {
	pub async fn new(runner: &render::Runner) -> Result<Self, Error> {
		let project = Project::empty();
		let egui = render::egui::Context::default();

		let (state, window) = render::State::new(&project.name, &runner, &egui).await?;
		let state = State::new(state);
		let state = Arc::new(state);

		window.set_window_icon(include_bytes!("../assets/png/tree-fill-big.png"));

		#[cfg(windows)]
		window.set_taskbar_icon(include_bytes!("../assets/png/tree-fill-big.png"));

		let tree = Tree::new(
			state.clone(),
			&project,
			None,
			project.properties[0].clone(),
			&window,
		);

		Ok(Self {
			window,
			egui,
			game: Game {
				paused: false,
				tree,
				custom_state: CustomState {
					project,
					project_time: std::time::SystemTime::now(),
					path: None,
				},
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
			},
		})
	}
}

impl Game<CustomState> {
	fn change_project(&mut self, window: &render::Window) {
		let Some(path) = rfd::FileDialog::new()
			.add_filter("Project File", &["epc"])
			.pick_file()
		else {
			return;
		};
		self.custom_state.path = Some(path);
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
		let Some(path) = &self.path else {
			return self.project_time;
		};
		path.metadata()
			.map(|meta| meta.modified().unwrap_or(self.project_time))
			.unwrap_or(self.project_time)
	}

	fn reload(&mut self, project_time: std::time::SystemTime, window: &render::Window) {
		self.custom_state.project_time = project_time;
		let Some(path) = &self.custom_state.path else {
			return;
		};
		self.custom_state.project = Project::from_file(path);
		self.tree = Tree::new(
			self.state.clone(),
			&self.project,
			Some(path.parent().unwrap().to_owned()),
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
		let Some(path) = &self.path else {
			return;
		};
		let path = path.parent().unwrap().to_path_buf();
		let mut reader = Reader::new(path, "segment");
		if let Some(segment) = self.tree.raycast(start, direction, &mut reader) {
			self.tree.segment = Some(Segment::new(&self.state, &mut self.tree.segments, segment));
			window.request_redraw();
		}
	}
}

impl Game<CustomState> {
	fn ui(&mut self, ctx: &render::egui::Context, window: &mut render::Window) {
		const HEIGHT: f32 = 10.0;
		const LEFT: f32 = 100.0;
		const RIGHT: f32 = 150.0;

		use render::egui::*;

		let full = Layout::top_down_justified(Align::Center);
		SidePanel::left("left")
			.resizable(false)
			.default_width(275.0)
			.show(ctx, |ui| {
				if ui
					.add_sized([ui.available_width(), HEIGHT], Button::new("Load Project"))
					.clicked()
				{
					self.change_project(window);
				};
				ui.separator();

				if ui
					.add_sized(
						[ui.available_width(), HEIGHT],
						SelectableLabel::new(self.property_options, "Property"),
					)
					.clicked()
				{
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
								for prop in &self.custom_state.project.properties {
									changed |= ui
										.selectable_value(&mut self.tree.property.0, prop.0.clone(), &prop.1)
										.changed();
								}
								if changed {
									for prop in &self.custom_state.project.properties {
										if prop.0 == self.tree.property.0 {
											self.tree.property = prop.clone();
										}
									}
									self.tree
										.loaded_manager
										.change_property(&self.tree.property.0);
									self.tree.segments.change_property(&self.tree.property.0);
									self.tree.update_lookup(&self.state);
									if let Some(seg) = &mut self.tree.segment {
										seg.change_property(&self.state, &mut self.tree.segments);
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
						if ui
							.add_sized(
								[ui.available_width() / 3.0, HEIGHT],
								SelectableLabel::new(seg.render == MeshRender::Points, "Points"),
							)
							.clicked()
						{
							seg.render = MeshRender::Points;
						}
						ui.set_enabled(matches!(
							seg.mesh,
							segment::MeshState::Done(..) | segment::MeshState::Progress(..)
						));
						if ui
							.add_sized(
								[ui.available_width() / 2.0, HEIGHT],
								SelectableLabel::new(seg.render == MeshRender::Mesh, "Mesh"),
							)
							.clicked()
						{
							seg.render = MeshRender::Mesh;
						}
						if ui
							.add_sized(
								[ui.available_width() / 1.0, HEIGHT],
								SelectableLabel::new(seg.render == MeshRender::MeshLines, "Lines"),
							)
							.clicked()
						{
							seg.render = MeshRender::MeshLines;
						}
					});

					ui.group(|ui| {
						ui.horizontal(|ui| {
							ui.add_sized([LEFT, HEIGHT], Label::new("Triangualation"));
							if ui
								.add_sized([ui.available_width(), HEIGHT], Button::new("Start"))
								.clicked()
							{
								seg.triangulate(&self.state);
								if seg.render == MeshRender::Points {
									seg.render = MeshRender::Mesh;
								}
							};
						});

						ui.horizontal(|ui| {
							ui.add_sized([LEFT, HEIGHT], Label::new("Alpha"));
							ui.add_sized(
								[ui.available_width(), HEIGHT],
								DragValue::new(&mut seg.alpha)
									.clamp_range(0.01..=10.0)
									.speed(0.002),
							);
						});

						ui.horizontal(|ui| {
							ui.add_sized([LEFT, HEIGHT], Label::new("Subsample"));
							ui.add_sized(
								[ui.available_width(), HEIGHT],
								DragValue::new(&mut seg.sub_sample_distance)
									.clamp_range(0.01..=1.0)
									.speed(0.002),
							);
						});
					});

					for (index, info) in self
						.custom_state
						.project
						.segment(seg.index())
						.iter()
						.enumerate()
					{
						ui.horizontal(|ui| {
							ui.add_sized(
								[LEFT, HEIGHT],
								Label::new(&self.custom_state.project.segment_information[index]),
							);
							ui.add_sized([LEFT, HEIGHT], Label::new(format!("{}", info)));
						});
					}

					ui.horizontal(|ui| {
						ui.add_sized([LEFT, HEIGHT], Label::new("Points"));
						if ui.add_sized([RIGHT, HEIGHT], Button::new("Save")).clicked() {
							seg.save();
						};
					});
				}
				ui.separator();

				if ui
					.add_sized(
						[ui.available_width(), HEIGHT],
						SelectableLabel::new(self.visual_options, "Visual"),
					)
					.clicked()
				{
					self.visual_options = self.visual_options.not();
				}
				if self.visual_options {
					ui.horizontal(|ui| {
						ui.add_sized([LEFT, HEIGHT], Label::new("Point Size"));
						if ui
							.add_sized(
								[RIGHT, HEIGHT],
								Slider::new(&mut self.tree.environment.scale, 0.0..=2.0),
							)
							.changed()
						{
							self.tree.environment = render::PointCloudEnvironment::new(
								&self.state,
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
						if ui
							.add_sized([RIGHT, HEIGHT], Slider::new(&mut min, 0.0..=1.0))
							.changed()
						{
							self.tree.environment.min = (min * u32::MAX as f32) as u32;
							self.tree.environment.max = self.tree.environment.max.max(self.tree.environment.min);
							self.tree.environment = render::PointCloudEnvironment::new(
								&self.state,
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
						if ui
							.add_sized([RIGHT, HEIGHT], Slider::new(&mut max, 0.0..=1.0))
							.changed()
						{
							self.tree.environment.max = (max * u32::MAX as f32) as u32;
							self.tree.environment.min = self.tree.environment.min.min(self.tree.environment.max);
							self.tree.environment = render::PointCloudEnvironment::new(
								&self.state,
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
								changed |= ui
									.selectable_value(&mut self.tree.lookup_name, LookupName::Warm, "Warm")
									.changed();
								changed |= ui
									.selectable_value(&mut self.tree.lookup_name, LookupName::Cold, "Cold")
									.changed();
								changed |= ui
									.selectable_value(&mut self.tree.lookup_name, LookupName::Turbo, "Turbo")
									.changed();
								if changed {
									self.tree.update_lookup(&self.state);
								}
							});
					});

					ui.horizontal(|ui| {
						ui.add_sized([LEFT, HEIGHT], Label::new("Background"));
						ui.style_mut().spacing.interact_size.x = ui.available_width();
						if ui
							.color_edit_button_rgb(self.tree.background.data_mut())
							.changed()
						{
							window.request_redraw();
						}
					});

					ui.horizontal(|ui| {
						ui.add_sized([LEFT, HEIGHT], Label::new("Screenshot"));
						if ui
							.add_sized(
								[RIGHT, HEIGHT],
								Button::new("Save").min_size(ui.available_size()),
							)
							.clicked()
						{
							let Some(path) = rfd::FileDialog::new()
								.add_filter("PNG", &["png"])
								.save_file()
							else {
								return;
							};
							window.screen_shot(self.state.deref(), &mut self.tree, path);
							window.request_redraw()
						}
					});

					ui.horizontal(|ui| {
						ui.add_sized([LEFT, HEIGHT], Label::new("Debug"));
						if ui
							.add_sized(
								[ui.available_width(), HEIGHT],
								SelectableLabel::new(self.tree.voxels_active, "Voxels"),
							)
							.clicked()
						{
							self.tree.voxels_active = self.tree.voxels_active.not();
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
						if ui
							.add_sized(
								[RIGHT, HEIGHT],
								Slider::new(&mut self.tree.eye_dome.strength, 0.0..=1.0),
							)
							.changed()
						{
							self.tree.eye_dome.update_settings(&self.state);
						}
					});

					ui.horizontal(|ui| {
						ui.add_sized([LEFT, HEIGHT], Label::new("Color"));
						ui.style_mut().spacing.interact_size.x = ui.available_size().x;
						if ui
							.color_edit_button_rgb(self.tree.eye_dome.color.data_mut())
							.changed()
						{
							self.tree.eye_dome.update_settings(&self.state);
						}
					});
				};
				ui.separator();

				if ui
					.add_sized(
						[ui.available_width(), HEIGHT],
						SelectableLabel::new(self.level_of_detail_options, "Level of Detail"),
					)
					.clicked()
				{
					self.level_of_detail_options = self.level_of_detail_options.not();
				}
				if self.level_of_detail_options {
					ui.horizontal(|ui| {
						ui.add_sized([LEFT, HEIGHT], Label::new("Mode"));
						ComboBox::from_id_source("level_of_detail_mode")
							.width(RIGHT)
							.selected_text(match self.tree.camera.lod {
								lod::Mode::Auto { .. } => "Automatic",
								lod::Mode::Normal { .. } => "Distance",
								lod::Mode::Level { .. } => "Level",
							})
							.show_ui(ui, |ui| {
								ui.selectable_value(
									&mut self.tree.camera.lod,
									lod::Mode::new_auto(),
									"Automatic",
								);
								ui.selectable_value(
									&mut self.tree.camera.lod,
									lod::Mode::new_normal(),
									"Distance",
								);
								ui.selectable_value(
									&mut self.tree.camera.lod,
									lod::Mode::new_level(self.custom_state.project.depth as usize),
									"Level",
								);
							});
					});

					match &mut self.tree.camera.lod {
						lod::Mode::Auto { threshold, target } => {
							ui.horizontal(|ui| {
								ui.add_sized([LEFT, HEIGHT], Label::new("Target FPS"));
								ui.add_sized([RIGHT, HEIGHT], Slider::new(target, 10.0..=120.0));
							});
							ui.set_enabled(false);
							ui.horizontal(|ui| {
								ui.add_sized([LEFT, HEIGHT], Label::new("Precision"));
								ui.add_sized([RIGHT, HEIGHT], Slider::new(threshold, 0.0..=10.0));
							});
						},
						lod::Mode::Normal { threshold } => {
							ui.horizontal(|ui| {
								ui.add_sized([LEFT, HEIGHT], Label::new("Precision"));
								ui.add_sized([RIGHT, HEIGHT], Slider::new(threshold, 0.0..=10.0));
							});
						},
						lod::Mode::Level { target, max } => {
							ui.horizontal(|ui| {
								ui.add_sized([LEFT, HEIGHT], Label::new("Level"));
								ui.add_sized([RIGHT, HEIGHT], Slider::new(target, 0..=*max));
							});
						},
					};
				}
				ui.separator();

				if ui
					.add_sized(
						[ui.available_width(), HEIGHT],
						SelectableLabel::new(self.camera_options, "Camera"),
					)
					.clicked()
				{
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
						ui.add_sized(
							[LEFT, HEIGHT],
							Label::new(match self.tree.camera.controller {
								camera::Controller::Orbital { .. } => "Distance",
								camera::Controller::FirstPerson { .. } => "Speed",
							}),
						);
						match &mut self.tree.camera.controller {
							camera::Controller::Orbital { offset } => {
								let old = *offset;
								ui.add_sized([RIGHT, HEIGHT], DragValue::new(offset));
								if *offset < 0.1 {
									*offset = 0.1;
								}
								let diff = *offset - old;
								if diff.abs() > 0.001 {
									self.tree.camera.move_in_view_direction(diff, &self.state);
								}
							},
							camera::Controller::FirstPerson { sensitivity } => {
								ui.add_sized([RIGHT, HEIGHT], DragValue::new(sensitivity));
								if *sensitivity < 0.01 {
									*sensitivity = 0.01;
								}
							},
						};
					});

					ui.horizontal_top(|ui| {
						ui.add_sized([LEFT, HEIGHT], Label::new("Position"));
						if ui
							.add_sized([ui.available_width() / 2.0, HEIGHT], Button::new("Save"))
							.clicked()
						{
							self.tree.camera.save();
						}
						if ui
							.add_sized([ui.available_width(), HEIGHT], Button::new("Load"))
							.clicked()
						{
							self.tree.camera.load(&self.state);
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
		let full_output = self
			.egui
			.run(raw_input, |ctx| self.game.ui(ctx, &mut self.window));

		self.window.render(
			self.game.state.deref(),
			&mut self.game.tree,
			full_output,
			&self.egui,
		);
	}

	fn resize_window(&mut self, _window_id: render::WindowId, size: Vector<2, u32>) {
		self.paused = size[X] == 0 || size[Y] == 0;
		if self.paused {
			return;
		}
		self.window.resized(self.game.state.deref());
		self.game
			.tree
			.camera
			.cam
			.set_aspect(self.window.get_aspect());
		self.tree.camera.gpu = render::Camera3DGPU::new(
			&self.state,
			&self.tree.camera.cam,
			&self.tree.camera.transform,
		);
		self.game
			.tree
			.eye_dome
			.update_depth(&self.game.state, self.window.depth_texture());
	}

	fn request_redraw(&mut self) {
		self.window.request_redraw();
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
			self.game.tree.camera.movement(direction, &self.game.state);
		}

		if self.tree.loaded_manager.update().not() && self.tree.segment.is_none() {
			self.game.tree.camera.time(delta.as_secs_f32())
		}

		if let Some(segment) = &mut self.game.tree.segment {
			if matches!(segment.render, MeshRender::Mesh | MeshRender::MeshLines) {
				segment.update(&self.game.state);
			}
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
		self.game.tree.camera.scroll(delta, &self.game.state);
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
			_ => {},
		}
	}

	fn mouse_moved(&mut self, _window_id: render::WindowId, position: Vector<2, f32>) {
		let delta = self.mouse.delta(position);
		if self.mouse.pressed(input::MouseButton::Left) {
			self.game.tree.camera.rotate(delta, &self.game.state);
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
