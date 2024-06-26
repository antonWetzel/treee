use std::{
	ops::{Deref, Not},
	path::PathBuf,
	sync::Arc,
};

use nalgebra as na;
use project::Project;

use crate::{
	camera, lod,
	reader::Reader,
	segment::{self, MeshRender, Segment},
	state::State,
	tree::{LookupName, Tree},
	Error, EventLoop,
};

pub struct World {
	window: render::Window,

	tree: Tree,
	project: Project,
	path: Option<PathBuf>,
	project_time: std::time::SystemTime,

	state: Arc<State>,
	mouse: input::Mouse,
	mouse_start: Option<na::Point2<f32>>,

	egui: egui::Context,
	egui_winit: egui_winit::State,
	egui_wgpu: egui_wgpu::Renderer,

	keyboard: input::Keyboard,
	time: Time,
	paused: bool,

	property_options: bool,
	visual_options: bool,
	level_of_detail_options: bool,
	camera_options: bool,
	quit: bool,
}

impl World {
	pub async fn new(event_loop: &EventLoop) -> Result<Self, Error> {
		let project = Project::empty();
		let egui = egui::Context::default();

		let (state, window) = render::State::new(&project.name, event_loop).await?;
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

		let id = egui.viewport_id();
		let egui_wgpu = egui_wgpu::Renderer::new(state.device(), state.surface_format(), None, 1);
		let egui_winit = egui_winit::State::new(egui.clone(), id, window.deref(), None, None);

		Ok(Self {
			window,
			egui,
			paused: false,

			tree,
			project,
			project_time: std::time::SystemTime::now(),
			path: None,

			state,
			mouse: input::Mouse::new(),
			mouse_start: None,
			keyboard: input::Keyboard::new(),
			time: Time::new(),

			egui_wgpu,
			egui_winit,

			property_options: false,
			level_of_detail_options: false,
			camera_options: false,
			visual_options: false,
			quit: false,
		})
	}

	fn change_project(&mut self) {
		let Some(path) = rfd::FileDialog::new()
			.add_filter("Project File", &["json"])
			.pick_file()
		else {
			return;
		};
		self.path = Some(path);
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
		let Some(path) = &self.path else {
			return self.project_time;
		};
		path.metadata()
			.map(|meta| meta.modified().unwrap_or(self.project_time))
			.unwrap_or(self.project_time)
	}

	fn reload(&mut self, project_time: std::time::SystemTime) {
		let Some(path) = &self.path else {
			return;
		};
		let Some(project) = Project::from_file(path) else {
			return;
		};
		self.project_time = project_time;
		self.project = project;
		self.tree = Tree::new(
			self.state.clone(),
			&self.project,
			Some(path.parent().unwrap().to_owned()),
			self.project.properties[0].clone(),
			&self.window,
		);
		self.window.set_title(&self.project.name);
		self.window.request_redraw();
	}

	fn raycast(&mut self) {
		if self.tree.segment.is_some() {
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
		let Some(path) = &self.path else {
			return;
		};
		let path = path.parent().unwrap().to_path_buf();
		let mut reader = Reader::new(path, "segment");
		if let Some(segment) = self.tree.raycast(start, direction, &mut reader) {
			self.tree.segment = Some(Segment::new(&self.state, &mut self.tree.segments, segment));
		}
	}

	fn ui(&mut self, ctx: &egui::Context) {
		const HEIGHT: f32 = 10.0;
		const LEFT: f32 = 100.0;
		const RIGHT: f32 = 150.0;

		use egui::*;

		let full = Layout::top_down_justified(Align::Center);

		SidePanel::left("left")
			.resizable(false)
			.default_width(275.0)
			.show(ctx, |ui| {
				if ui
					.add_sized([ui.available_width(), HEIGHT], Button::new("Load Project"))
					.clicked()
				{
					self.change_project();
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
							.selected_text(&self.tree.property.display_name)
							.show_ui(ui, |ui| {
								let mut changed = false;
								for prop in &self.project.properties {
									changed |= ui
										.selectable_value(
											&mut self.tree.property.storage_name,
											prop.storage_name.clone(),
											&prop.display_name,
										)
										.changed();
								}
								if changed {
									for prop in &self.project.properties {
										if prop.storage_name == self.tree.property.storage_name {
											self.tree.property = prop.clone();
										}
									}
									self.tree
										.loaded_manager
										.change_property(&self.tree.property.storage_name);
									self.tree
										.segments
										.change_property(&self.tree.property.storage_name);
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
					ui.add_enabled_ui(seg, |ui| {
						if ui.toggle_value(&mut seg, "Segment").changed() && seg.not() {
							self.tree.segment = None;
						}
					});
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
						let enabled = matches!(
							seg.mesh,
							segment::MeshState::Done(..) | segment::MeshState::Progress(..)
						);
						ui.add_enabled_ui(enabled, |ui| {
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
					});
					ui.horizontal(|ui| {
						ui.add_sized([LEFT, HEIGHT], Label::new(""));
						if ui
							.add_sized([RIGHT, HEIGHT], SelectableLabel::new(seg.show_grid, "Grid"))
							.clicked()
						{
							seg.show_grid = !seg.show_grid;
						};
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

					for (index, info) in self.project.segment(seg.index()).iter().enumerate() {
						ui.horizontal(|ui| {
							ui.add_sized(
								[LEFT, HEIGHT],
								Label::new(&self.project.segment_information[index]),
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
						}
					});

					ui.horizontal(|ui| {
						ui.add_sized([LEFT, HEIGHT], Label::new("Min"));
						let mut min = self.tree.environment.min as f32 / self.tree.property.max as f32;
						if ui
							.add_sized([RIGHT, HEIGHT], Slider::new(&mut min, 0.0..=1.0))
							.changed()
						{
							self.tree.environment.min = (min * self.tree.property.max as f32) as u32;
							self.tree.environment.max = self.tree.environment.max.max(self.tree.environment.min);
							self.tree.environment = render::PointCloudEnvironment::new(
								&self.state,
								self.tree.environment.min,
								self.tree.environment.max,
								self.tree.environment.scale,
							);
						}
					});

					ui.horizontal(|ui| {
						ui.add_sized([LEFT, HEIGHT], Label::new("Max"));
						let mut max = self.tree.environment.max as f32 / self.tree.property.max as f32;
						if ui
							.add_sized([RIGHT, HEIGHT], Slider::new(&mut max, 0.0..=1.0))
							.changed()
						{
							self.tree.environment.max = (max * self.tree.property.max as f32) as u32;
							self.tree.environment.min = self.tree.environment.min.min(self.tree.environment.max);
							self.tree.environment = render::PointCloudEnvironment::new(
								&self.state,
								self.tree.environment.min,
								self.tree.environment.max,
								self.tree.environment.scale,
							);
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
						ui.color_edit_button_rgb(&mut self.tree.background.coords.data.0[0]);
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
							let background = self.tree.background;
							self.window.screen_shot(
								&self.state,
								&mut (self.state.deref(), &self.tree, &mut self.egui_wgpu),
								|_, _| {},
								|(state, tree, _), render_pass| tree.render(state, render_pass),
								|(state, tree, _), render_pass| tree.post_process(state, render_pass),
								background,
								path,
							);
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
							.color_edit_button_rgb(&mut self.tree.eye_dome.color.coords.data.0[0])
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
									lod::Mode::new_level(self.project.depth as usize),
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
							ui.add_enabled_ui(false, |ui| {
								ui.horizontal(|ui| {
									ui.add_sized([LEFT, HEIGHT], Label::new("Precision"));
									ui.add_sized([RIGHT, HEIGHT], Slider::new(threshold, 0.0..=10.0));
								});
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
						ui.add_sized([LEFT, HEIGHT], Label::new("Field of View"));
						let mut fovy = self.tree.camera.cam.fovy.to_degrees();
						let changed = ui
							.add_sized(
								[RIGHT, HEIGHT],
								DragValue::new(&mut fovy).clamp_range(10..=130),
							)
							.changed();
						if changed {
							self.tree.camera.cam.fovy = fovy.to_radians();
							self.tree.camera.update_gpu(&self.state);
						}
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
						}
					});
				}
			});
	}

	pub fn raw_event(&mut self, event: &winit::event::WindowEvent) -> bool {
		self.egui_winit
			.on_window_event(&self.window, event)
			.consumed
	}

	pub fn render(&mut self, _window_id: render::WindowId) {
		if self.paused {
			return;
		}
		if self.tree.segment.is_none() {
			self.tree.update();
		}

		let raw_input = self.egui_winit.take_egui_input(&self.window);

		let full_output = self.egui.clone().run(raw_input, |ctx| self.ui(ctx));

		self.egui_winit
			.handle_platform_output(&self.window, full_output.platform_output);
		let paint_jobs = self
			.egui
			.tessellate(full_output.shapes, full_output.pixels_per_point);

		let config = self.window.config();
		let screen = &egui_wgpu::ScreenDescriptor {
			size_in_pixels: [config.width, config.height],
			pixels_per_point: 1.0,
		};
		for (id, delta) in full_output.textures_delta.set {
			self.egui_wgpu
				.update_texture(&self.state.device, &self.state.queue, id, &delta);
		}
		for id in full_output.textures_delta.free {
			self.egui_wgpu.free_texture(&id);
		}

		let background = self.tree.background;

		self.window.render(
			self.state.deref(),
			&mut (self.state.deref(), &self.tree, &mut self.egui_wgpu),
			|(state, _, egui_wgpu), command_encoder| {
				let commands = egui_wgpu.update_buffers(
					&state.device,
					&state.queue,
					command_encoder,
					&paint_jobs,
					screen,
				);
				state.queue.submit(commands);
			},
			|(state, tree, _), render_pass| tree.render(state, render_pass),
			|(state, tree, egui_wgpu), render_pass| {
				tree.post_process(state, render_pass);
				egui_wgpu.render(render_pass, &paint_jobs, screen);
			},
			background,
		);
	}

	pub fn resize_window(&mut self, _window_id: render::WindowId, size: na::Point2<u32>) {
		self.paused = size.x == 0 || size.y == 0;
		if self.paused {
			return;
		}
		self.window.resized(self.state.deref());
		self.tree.camera.cam.aspect = self.window.get_aspect();
		self.tree.camera.update_gpu(&self.state);
		self.tree
			.eye_dome
			.update_depth(&self.state, self.window.depth_texture());
	}

	pub fn request_redraw(&mut self) {
		self.window.request_redraw();
	}

	pub fn close_window(&mut self, _window_id: render::WindowId) {
		self.quit = true;
	}

	pub fn time(&mut self) {
		let delta = self.time.elapsed();
		let mut direction = na::vector![0.0, 0.0];
		if self.keyboard.pressed(input::KeyCode::KeyD) || self.keyboard.pressed(input::KeyCode::ArrowRight) {
			direction.x += 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyS) || self.keyboard.pressed(input::KeyCode::ArrowDown) {
			direction.y += 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyA) || self.keyboard.pressed(input::KeyCode::ArrowLeft) {
			direction.x -= 1.0;
		}
		if self.keyboard.pressed(input::KeyCode::KeyW) || self.keyboard.pressed(input::KeyCode::ArrowUp) {
			direction.y -= 1.0;
		}
		let l = direction.norm();
		if l > 0.0 {
			direction *= 10.0 * delta.as_secs_f32() / l;
			self.tree.camera.movement(direction, &self.state);
		}

		if self.tree.loaded_manager.update().not() && self.tree.segment.is_none() {
			self.tree.camera.time(delta.as_secs_f32())
		}

		if let Some(segment) = &mut self.tree.segment {
			if matches!(segment.render, MeshRender::Mesh | MeshRender::MeshLines) {
				segment.update(&self.state);
			}
		}

		self.check_reload();
	}

	pub fn key_changed(&mut self, _window_id: render::WindowId, key: input::KeyCode, key_state: input::State) {
		self.keyboard.update(key, key_state);
	}

	pub fn modifiers_changed(&mut self, modifiers: input::Modifiers) {
		self.keyboard.update_modifiers(modifiers);
	}

	pub fn mouse_wheel(&mut self, delta: f32) {
		self.tree.camera.scroll(delta, &self.state);
	}

	pub fn mouse_button_changed(
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
					let dist = (start - self.mouse.position()).norm();
					if dist < 2.0 {
						self.raycast();
					}
				}
			},
			_ => {},
		}
	}

	pub fn mouse_moved(&mut self, _window_id: render::WindowId, position: na::Point2<f32>) {
		let delta = self.mouse.delta(position);
		if self.mouse.pressed(input::MouseButton::Left) {
			self.tree.camera.rotate(delta, &self.state);
		}
	}

	pub fn exit(&self) -> bool {
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
