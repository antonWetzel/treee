use std::{
	ops::{Deref, Not},
	path::PathBuf,
	sync::Arc,
};

use math::{Vector, X, Y};
use project::Project;
use render::Window;
use window::{
	camera, lod,
	tree::{LookupName, Tree},
	Game, State,
};

use crate::{
	loaded_manager::LoadedManager,
	reader::Reader,
	segment::{self, MeshRender, Segment},
	tree::{Node, ProjectScene},
	Error,
};

pub struct World {
	game: Game<ProjectCustomState>,
	egui: render::egui::Context,
}

impl std::ops::Deref for World {
	type Target = Game<ProjectCustomState>;

	fn deref(&self) -> &Self::Target {
		&self.game
	}
}

impl std::ops::DerefMut for World {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.game
	}
}

pub struct ProjectCustomState {
	project: Project,
	path: Option<PathBuf>,
	project_time: std::time::SystemTime,
}

impl window::CustomState for ProjectCustomState {
	type Scene = ProjectScene;
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

		let tree = Game::<ProjectCustomState>::new_tree(
			state.clone(),
			&project,
			None,
			project.properties[0].clone(),
			&window,
		);

		Ok(Self {
			egui,
			game: Game::new(
				window,
				tree,
				state,
				ProjectCustomState {
					project,
					project_time: std::time::SystemTime::now(),
					path: None,
				},
			),
		})
	}
}

trait GameExt {
	fn new_tree(
		state: Arc<State>,
		project: &Project,
		path: Option<PathBuf>,
		property: (String, String, u32),
		window: &Window,
	) -> Tree<ProjectScene>;
	fn change_project(&mut self);

	fn check_reload(&mut self);

	fn current_project_time(&self) -> std::time::SystemTime;

	fn reload(&mut self, project_time: std::time::SystemTime);

	fn raycast(&mut self);
}

impl GameExt for Game<ProjectCustomState> {
	fn new_tree(
		state: Arc<State>,
		project: &Project,
		path: Option<PathBuf>,
		property: (String, String, u32),
		window: &Window,
	) -> Tree<ProjectScene> {
		let scene = ProjectScene {
			root: Node::new(&project.root, &state),
			segment: None,
			segments: path
				.clone()
				.map(|path| {
					let mut segments = path.clone();
					segments.push("segments");
					Reader::new(segments, &property.0)
				})
				.unwrap_or(Reader::fake()),
			loaded_manager: LoadedManager::new(state.clone(), path, &property.0),
		};
		Tree::new(state, property, window, scene)
	}
	fn change_project(&mut self) {
		let Some(path) = rfd::FileDialog::new()
			.add_filter("Project File", &["epc"])
			.pick_file()
		else {
			return;
		};
		self.custom_state.path = Some(path);
		self.reload(self.current_project_time());
	}

	fn check_reload(&mut self) {
		let project_time = self.current_project_time();
		if self.project_time == project_time {
			return;
		}
		if project_time.elapsed().unwrap() < std::time::Duration::from_millis(1000) {
			return;
		}
		self.reload(project_time);
	}

	fn current_project_time(&self) -> std::time::SystemTime {
		let Some(path) = &self.path else {
			return self.project_time;
		};
		path.metadata()
			.map(|meta| meta.modified().unwrap_or(self.project_time))
			.unwrap_or(self.project_time)
	}

	fn reload(&mut self, project_time: std::time::SystemTime) {
		self.custom_state.project_time = project_time;
		let Some(path) = &self.custom_state.path else {
			return;
		};
		self.custom_state.project = Project::from_file(path);
		self.tree = Self::new_tree(
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
		if self.tree.scene.segment.is_some() {
			return;
		}
		let start = self
			.tree
			.context
			.camera
			.ray_origin(self.mouse.position(), self.window.get_size());
		let direction = self
			.tree
			.context
			.camera
			.ray_direction(self.mouse.position(), self.window.get_size());
		let Some(path) = &self.path else {
			return;
		};
		let path = path.parent().unwrap().to_path_buf();
		let mut reader = Reader::new(path, "segment");
		if let Some(segment) = self.tree.scene.raycast(start, direction, &mut reader) {
			self.tree.scene.segment = Some(Segment::new(
				&self.state,
				&mut self.tree.scene.segments,
				segment,
			));
			self.window.request_redraw();
		}
	}
}

fn ui(ctx: &render::egui::Context, game: &mut Game<ProjectCustomState>) {
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
				game.change_project();
			};
			ui.separator();

			if ui
				.add_sized(
					[ui.available_width(), HEIGHT],
					SelectableLabel::new(game.property_options, "Property"),
				)
				.clicked()
			{
				game.property_options = game.property_options.not();
			};
			if game.property_options {
				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Selected"));
					ComboBox::from_id_source("property_selected")
						.width(RIGHT)
						.selected_text(&game.tree.context.property.1)
						.show_ui(ui, |ui| {
							let mut changed = false;
							for prop in &game.custom_state.project.properties {
								changed |= ui
									.selectable_value(&mut game.tree.context.property.0, prop.0.clone(), &prop.1)
									.changed();
							}
							if changed {
								for prop in &game.custom_state.project.properties {
									if prop.0 == game.tree.context.property.0 {
										game.tree.context.property = prop.clone();
									}
								}
								game.tree
									.scene
									.loaded_manager
									.change_property(&game.tree.context.property.0);
								game.tree
									.scene
									.segments
									.change_property(&game.tree.context.property.0);
								game.tree.context.update_lookup(&game.state);
								if let Some(seg) = &mut game.tree.scene.segment {
									seg.change_property(&game.state, &mut game.tree.scene.segments);
								}
							}
						});
				});
			}
			ui.separator();

			let mut seg = game.tree.scene.segment.is_some();
			ui.with_layout(full, |ui| {
				ui.set_enabled(seg);
				if ui.toggle_value(&mut seg, "Segment").changed() && seg.not() {
					game.tree.scene.segment = None;
				}
			});
			if let Some(seg) = &mut game.tree.scene.segment {
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
							seg.triangulate(&game.state);
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

				for (index, info) in game
					.custom_state
					.project
					.segment(seg.index())
					.iter()
					.enumerate()
				{
					ui.horizontal(|ui| {
						ui.add_sized(
							[LEFT, HEIGHT],
							Label::new(&game.custom_state.project.segment_information[index]),
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
					SelectableLabel::new(game.visual_options, "Visual"),
				)
				.clicked()
			{
				game.visual_options = game.visual_options.not();
			}
			if game.visual_options {
				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Point Size"));
					if ui
						.add_sized(
							[RIGHT, HEIGHT],
							Slider::new(&mut game.tree.context.environment.scale, 0.0..=2.0),
						)
						.changed()
					{
						game.tree.context.environment = render::PointCloudEnvironment::new(
							&game.state,
							game.tree.context.environment.min,
							game.tree.context.environment.max,
							game.tree.context.environment.scale,
						);
						game.window.request_redraw();
					}
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Min"));
					let mut min = game.tree.context.environment.min as f32 / u32::MAX as f32;
					if ui
						.add_sized([RIGHT, HEIGHT], Slider::new(&mut min, 0.0..=1.0))
						.changed()
					{
						game.tree.context.environment.min = (min * u32::MAX as f32) as u32;
						game.tree.context.environment.max = game
							.tree
							.context
							.environment
							.max
							.max(game.tree.context.environment.min);
						game.tree.context.environment = render::PointCloudEnvironment::new(
							&game.state,
							game.tree.context.environment.min,
							game.tree.context.environment.max,
							game.tree.context.environment.scale,
						);
						game.window.request_redraw();
					}
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Max"));
					let mut max = game.tree.context.environment.max as f32 / u32::MAX as f32;
					if ui
						.add_sized([RIGHT, HEIGHT], Slider::new(&mut max, 0.0..=1.0))
						.changed()
					{
						game.tree.context.environment.max = (max * u32::MAX as f32) as u32;
						game.tree.context.environment.min = game
							.tree
							.context
							.environment
							.min
							.min(game.tree.context.environment.max);
						game.tree.context.environment = render::PointCloudEnvironment::new(
							&game.state,
							game.tree.context.environment.min,
							game.tree.context.environment.max,
							game.tree.context.environment.scale,
						);
						game.window.request_redraw();
					}
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Color Palette"));
					ComboBox::from_id_source("color_palette")
						.selected_text(format!("{:?}", game.tree.context.lookup_name))
						.width(RIGHT)
						.show_ui(ui, |ui| {
							let mut changed = false;
							changed |= ui
								.selectable_value(&mut game.tree.context.lookup_name, LookupName::Warm, "Warm")
								.changed();
							changed |= ui
								.selectable_value(&mut game.tree.context.lookup_name, LookupName::Cold, "Cold")
								.changed();
							changed |= ui
								.selectable_value(
									&mut game.tree.context.lookup_name,
									LookupName::Turbo,
									"Turbo",
								)
								.changed();
							if changed {
								game.tree.context.update_lookup(&game.state);
							}
						});
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Background"));
					ui.style_mut().spacing.interact_size.x = ui.available_width();
					if ui
						.color_edit_button_rgb(game.tree.context.background.data_mut())
						.changed()
					{
						game.window.request_redraw();
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
						game.window
							.screen_shot(game.state.deref(), &mut game.tree, path);
						game.window.request_redraw()
					}
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Debug"));
					if ui
						.add_sized(
							[ui.available_width(), HEIGHT],
							SelectableLabel::new(game.tree.context.voxels_active, "Voxels"),
						)
						.clicked()
					{
						game.tree.context.voxels_active = game.tree.context.voxels_active.not();
					}
				});
			}
			ui.separator();

			ui.with_layout(full, |ui| {
				ui.toggle_value(&mut game.tree.context.eye_dome_active, "Eye Dome")
			});
			if game.tree.eye_dome_active {
				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Strength"));
					if ui
						.add_sized(
							[RIGHT, HEIGHT],
							Slider::new(&mut game.tree.context.eye_dome.strength, 0.0..=1.0),
						)
						.changed()
					{
						game.tree.context.eye_dome.update_settings(&game.state);
					}
				});

				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Color"));
					ui.style_mut().spacing.interact_size.x = ui.available_size().x;
					if ui
						.color_edit_button_rgb(game.tree.context.eye_dome.color.data_mut())
						.changed()
					{
						game.tree.context.eye_dome.update_settings(&game.state);
					}
				});
			};
			ui.separator();

			if ui
				.add_sized(
					[ui.available_width(), HEIGHT],
					SelectableLabel::new(game.level_of_detail_options, "Level of Detail"),
				)
				.clicked()
			{
				game.level_of_detail_options = game.level_of_detail_options.not();
			}
			if game.level_of_detail_options {
				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Mode"));
					ComboBox::from_id_source("level_of_detail_mode")
						.width(RIGHT)
						.selected_text(match game.tree.camera.lod {
							lod::Mode::Auto { .. } => "Automatic",
							lod::Mode::Normal { .. } => "Distance",
							lod::Mode::Level { .. } => "Level",
						})
						.show_ui(ui, |ui| {
							ui.selectable_value(
								&mut game.tree.context.camera.lod,
								lod::Mode::new_auto(),
								"Automatic",
							);
							ui.selectable_value(
								&mut game.tree.context.camera.lod,
								lod::Mode::new_normal(),
								"Distance",
							);
							ui.selectable_value(
								&mut game.tree.context.camera.lod,
								lod::Mode::new_level(game.custom_state.project.depth as usize),
								"Level",
							);
						});
				});

				match &mut game.tree.context.camera.lod {
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
					SelectableLabel::new(game.camera_options, "Camera"),
				)
				.clicked()
			{
				game.camera_options = game.camera_options.not();
			};
			if game.camera_options {
				ui.horizontal(|ui| {
					ui.add_sized([LEFT, HEIGHT], Label::new("Controller"));
					ComboBox::from_id_source("camera_controller")
						.width(RIGHT)
						.selected_text(match game.tree.camera.controller {
							camera::Controller::Orbital { .. } => "Orbital",
							camera::Controller::FirstPerson { .. } => "First Person",
						})
						.show_ui(ui, |ui| {
							let c = game.tree.camera.orbital();
							ui.selectable_value(&mut game.tree.context.camera.controller, c, "Orbital");
							let c = game.tree.camera.first_person();
							ui.selectable_value(&mut game.tree.context.camera.controller, c, "First Person");
						});
				});

				ui.horizontal(|ui| {
					ui.add_sized(
						[LEFT, HEIGHT],
						Label::new(match game.tree.camera.controller {
							camera::Controller::Orbital { .. } => "Distance",
							camera::Controller::FirstPerson { .. } => "Speed",
						}),
					);
					match &mut game.tree.context.camera.controller {
						camera::Controller::Orbital { offset } => {
							let old = *offset;
							ui.add_sized([RIGHT, HEIGHT], DragValue::new(offset));
							if *offset < 0.1 {
								*offset = 0.1;
							}
							let diff = *offset - old;
							if diff.abs() > 0.001 {
								game.tree
									.context
									.camera
									.move_in_view_direction(diff, &game.state);
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
						game.tree.camera.save();
					}
					if ui
						.add_sized([ui.available_width(), HEIGHT], Button::new("Load"))
						.clicked()
					{
						game.tree.context.camera.load(&game.state);
						game.window.request_redraw();
					}
				});
			}
		});
}

impl render::Entry for World {
	fn raw_event(&mut self, event: &render::Event) -> bool {
		self.game.raw_event(event)
	}

	fn render(&mut self, window_id: render::WindowId) {
		if self.paused {
			return;
		}
		if self.game.tree.scene.segment.is_none() {
			self.game.render(window_id);
		}

		let raw_input = self.game.take_egui_input();
		let full_output = self.egui.run(raw_input, |ctx| ui(ctx, &mut self.game));

		self.game.window.render(
			self.game.state.deref(),
			&mut self.game.tree,
			full_output,
			&self.egui,
		);
	}

	fn resize_window(&mut self, _window_id: render::WindowId, size: Vector<2, u32>) {
		self.game.resize_window(_window_id, size)
	}

	fn request_redraw(&mut self) {
		self.game.request_redraw();
	}

	fn close_window(&mut self, window_id: render::WindowId) {
		self.game.close_window(window_id);
	}

	fn time(&mut self) {
		let delta = self.time.elapsed();
		self.game.time_delta(delta);

		if self.tree.scene.loaded_manager.update().not() && self.tree.scene.segment.is_none() {
			self.game.tree.context.camera.time(delta.as_secs_f32())
		}

		if let Some(segment) = &mut self.game.tree.scene.segment {
			if matches!(segment.render, MeshRender::Mesh | MeshRender::MeshLines) {
				segment.update(&self.game.state);
			}
		}

		self.game.check_reload();
	}

	fn key_changed(&mut self, window_id: render::WindowId, key: input::KeyCode, key_state: input::State) {
		self.game.key_changed(window_id, key, key_state)
	}

	fn modifiers_changed(&mut self, modifiers: input::Modifiers) {
		self.game.modifiers_changed(modifiers)
	}

	fn mouse_wheel(&mut self, delta: f32) {
		self.game.mouse_wheel(delta)
	}

	fn mouse_button_changed(
		&mut self,
		window_id: render::WindowId,
		button: input::MouseButton,
		button_state: input::State,
	) {
		self.game
			.mouse_button_changed(window_id, button, button_state);
		match (button, button_state) {
			(input::MouseButton::Left, input::State::Pressed) => {
				self.mouse_start = Some(self.mouse.position());
			},
			(input::MouseButton::Left, input::State::Released) => {
				if let Some(start) = self.mouse_start {
					let dist = (start - self.mouse.position()).length();
					if dist < 2.0 {
						self.game.raycast();
					}
				}
			},
			_ => {},
		}
	}

	fn mouse_moved(&mut self, window_id: render::WindowId, position: Vector<2, f32>) {
		self.game.mouse_moved(window_id, position)
	}

	fn exit(&self) -> bool {
		self.game.exit()
	}
}
