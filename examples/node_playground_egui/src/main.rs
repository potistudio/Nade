use eframe::egui;
use egui::{ColorImage, TextureHandle, TextureOptions};
use nade_core::{
	EvalContext, Graph, Image, NodeId, NodeState, Operator, OutputType, ParamKind, ParamValue,
	Value, ValueKind, ValueParam, ValueParamUi,
};
use operators::{
	ImageBlur1DOp, ImageDelay1Op, ImageMixOp, ImageSolidColorOp, ValueAddOp, ValueCompareGTOp,
	ValueConstOp, ValueDelay1Op, ValueMulOp, ValueSelectOp, ValueSinOp,
};

#[derive(Debug, Clone)]
struct NodeEntry {
	id: NodeId,
	name: String,
}

#[derive(Debug, Clone)]
struct ValueNodeInfo {
	id: NodeId,
	name: String,
	kind: Option<ValueKind>,
	preview: Option<Value>,
}

#[derive(Debug, Clone)]
struct ImageNodeInfo {
	id: NodeId,
	name: String,
}

#[derive(Clone)]
struct EvalPreview {
	node_id: NodeId,
	texture: TextureHandle,
	size: [usize; 2],
}

#[derive(Debug, Clone)]
enum EvalOutput {
	Value(Value),
	Image([usize; 2]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperatorTemplate {
	ValueConst,
	ValueAdd,
	ValueMul,
	ValueSin,
	ValueCompareGT,
	ValueSelect,
	ValueDelay1,
	ImageSolidColor,
	ImageMix,
	ImageBlur1D,
	ImageDelay1,
}

impl OperatorTemplate {
	fn all() -> &'static [OperatorTemplate] {
		use OperatorTemplate::*;
		&[
			ValueConst,
			ValueAdd,
			ValueMul,
			ValueSin,
			ValueCompareGT,
			ValueSelect,
			ValueDelay1,
			ImageSolidColor,
			ImageMix,
			ImageBlur1D,
			ImageDelay1,
		]
	}

	fn label(self) -> &'static str {
		match self {
			Self::ValueConst => "Value.Const",
			Self::ValueAdd => "Value.Add",
			Self::ValueMul => "Value.Mul",
			Self::ValueSin => "Value.Sin",
			Self::ValueCompareGT => "Value.CompareGT",
			Self::ValueSelect => "Value.Select",
			Self::ValueDelay1 => "Value.Delay1",
			Self::ImageSolidColor => "Image.SolidColor",
			Self::ImageMix => "Image.Mix",
			Self::ImageBlur1D => "Image.Blur1D",
			Self::ImageDelay1 => "Image.Delay1",
		}
	}

	fn from_operator(operator: &dyn Operator) -> Option<Self> {
		if operator.as_any().is::<ValueConstOp>() {
			Some(Self::ValueConst)
		} else if operator.as_any().is::<ValueAddOp>() {
			Some(Self::ValueAdd)
		} else if operator.as_any().is::<ValueMulOp>() {
			Some(Self::ValueMul)
		} else if operator.as_any().is::<ValueSinOp>() {
			Some(Self::ValueSin)
		} else if operator.as_any().is::<ValueCompareGTOp>() {
			Some(Self::ValueCompareGT)
		} else if operator.as_any().is::<ValueSelectOp>() {
			Some(Self::ValueSelect)
		} else if operator.as_any().is::<ValueDelay1Op>() {
			Some(Self::ValueDelay1)
		} else if operator.as_any().is::<ImageSolidColorOp>() {
			Some(Self::ImageSolidColor)
		} else if operator.as_any().is::<ImageMixOp>() {
			Some(Self::ImageMix)
		} else if operator.as_any().is::<ImageBlur1DOp>() {
			Some(Self::ImageBlur1D)
		} else if operator.as_any().is::<ImageDelay1Op>() {
			Some(Self::ImageDelay1)
		} else {
			None
		}
	}
}

struct PlaygroundApp {
	graph: Graph,
	nodes: Vec<NodeEntry>,
	selected: Option<NodeId>,
	operator_to_add: OperatorTemplate,
	next_name_index: u64,
	time: f64,
	fps: f64,
	playing: bool,
	auto_eval: bool,
	eval_dirty: bool,
	last_eval: Option<EvalOutput>,
	last_error: Option<String>,
	preview: Option<EvalPreview>,
}

impl PlaygroundApp {
	fn new() -> Self {
		let mut app = Self {
			graph: Graph::new(),
			nodes: Vec::new(),
			selected: None,
			operator_to_add: OperatorTemplate::ValueConst,
			next_name_index: 1,
			time: 0.0,
			fps: 60.0,
			playing: false,
			auto_eval: true,
			eval_dirty: true,
			last_eval: None,
			last_error: None,
			preview: None,
		};

		app.build_default_graph();
		app.selected = app.nodes.first().map(|node| node.id);
		app
	}

	fn build_default_graph(&mut self) {
		let freq = self.add_node_named("freq", ValueConstOp::new(Value::Float(0.5)));
		let phase = self.add_node_named("phase", ValueConstOp::new(Value::Float(0.0)));
		let one = self.add_node_named("one", ValueConstOp::new(Value::Float(1.0)));
		let half = self.add_node_named("half", ValueConstOp::new(Value::Float(0.5)));

		let sin = self.add_node_named(
			"sin",
			ValueSinOp::new(
				ValueParam::with_mapping(Value::Float(0.0), freq),
				ValueParam::with_mapping(Value::Float(0.0), phase),
			),
		);
		let add = self.add_node_named(
			"sin_plus_one",
			ValueAddOp::new(
				ValueParam::with_mapping(Value::Float(0.0), sin),
				ValueParam::with_mapping(Value::Float(0.0), one),
			),
		);
		let lfo = self.add_node_named(
			"lfo",
			ValueMulOp::new(
				ValueParam::with_mapping(Value::Float(0.0), add),
				ValueParam::with_mapping(Value::Float(0.0), half),
			),
		);

		let blur_base = self.add_node_named("blur_base", ValueConstOp::new(Value::Float(1.0)));
		let blur_scale = self.add_node_named("blur_scale", ValueConstOp::new(Value::Float(10.0)));
		let blur_scale_mul = self.add_node_named(
			"blur_scale_mul",
			ValueMulOp::new(
				ValueParam::with_mapping(Value::Float(0.0), lfo),
				ValueParam::with_mapping(Value::Float(0.0), blur_scale),
			),
		);
		let blur_radius = self.add_node_named(
			"blur_radius",
			ValueAddOp::new(
				ValueParam::with_mapping(Value::Float(0.0), blur_base),
				ValueParam::with_mapping(Value::Float(0.0), blur_scale_mul),
			),
		);

		let color_a = self.add_node_named(
			"color_a",
			ValueConstOp::new(Value::Vec4([1.0, 0.2, 0.2, 1.0])),
		);
		let color_b = self.add_node_named(
			"color_b",
			ValueConstOp::new(Value::Vec4([0.2, 0.4, 1.0, 1.0])),
		);

		let img_a = self.add_node_named(
			"image_a",
			ImageSolidColorOp::new(
				ValueParam::with_mapping(Value::Vec4([0.0, 0.0, 0.0, 0.0]), color_a),
				256,
				256,
			),
		);
		let img_b = self.add_node_named(
			"image_b",
			ImageSolidColorOp::new(
				ValueParam::with_mapping(Value::Vec4([0.0, 0.0, 0.0, 0.0]), color_b),
				256,
				256,
			),
		);

		let mix = self.add_node_named(
			"mix",
			ImageMixOp::new(
				img_a,
				img_b,
				ValueParam::with_mapping(Value::Float(0.0), lfo),
			),
		);

		self.add_node_named(
			"blur",
			ImageBlur1DOp::new(
				mix,
				ValueParam::with_mapping(Value::Float(0.0), blur_radius),
			),
		);
	}

	fn add_node_named<O>(&mut self, name: impl Into<String>, operator: O) -> NodeId
	where
		O: Operator + 'static,
	{
		let node_id = self.graph.add_node(operator);
		self.nodes.push(NodeEntry {
			id: node_id,
			name: name.into(),
		});
		node_id
	}

	fn add_node_with_operator(&mut self, template: OperatorTemplate) -> NodeId {
		let name = format!("{} {}", template.label(), self.next_name_index);
		self.next_name_index += 1;
		let operator = self.make_operator(template);
		let node_id = self.add_node_named_boxed(name, operator);
		self.selected = Some(node_id);
		self.eval_dirty = true;
		node_id
	}

	fn add_auto_node(&mut self, template: OperatorTemplate) -> NodeId {
		let name = format!("Auto {} {}", template.label(), self.next_name_index);
		self.next_name_index += 1;
		let operator = self.make_operator(template);
		let node_id = self.add_node_named_boxed(name, operator);
		self.eval_dirty = true;
		node_id
	}

	fn add_node_named_boxed(
		&mut self,
		name: impl Into<String>,
		operator: Box<dyn Operator>,
	) -> NodeId {
		let node_id = self.graph.add_node_boxed(operator);
		self.nodes.push(NodeEntry {
			id: node_id,
			name: name.into(),
		});
		node_id
	}

	fn remove_node(&mut self, node_id: NodeId) {
		self.graph.remove_node(node_id);
		self.nodes.retain(|node| node.id != node_id);
		if self.selected == Some(node_id) {
			self.selected = None;
		}
		if let Some(preview) = &self.preview
			&& preview.node_id == node_id
		{
			self.preview = None;
		}
		self.eval_dirty = true;
	}

	fn apply_operator_change(&mut self, node_id: NodeId, template: OperatorTemplate) {
		let operator = self.make_operator(template);
		if let Some(node) = self.graph.node_mut(node_id) {
			node.operator = operator;
			node.state = NodeState::default();
		}
		self.graph.clear_cache();
		self.eval_dirty = true;
	}

	fn make_operator(&mut self, template: OperatorTemplate) -> Box<dyn Operator> {
		match template {
			OperatorTemplate::ValueConst => Box::new(ValueConstOp::new(Value::Float(0.0))),
			OperatorTemplate::ValueAdd => Box::new(ValueAddOp::new(
				ValueParam::new(Value::Float(0.0)),
				ValueParam::new(Value::Float(0.0)),
			)),
			OperatorTemplate::ValueMul => Box::new(ValueMulOp::new(
				ValueParam::new(Value::Float(1.0)),
				ValueParam::new(Value::Float(1.0)),
			)),
			OperatorTemplate::ValueSin => Box::new(ValueSinOp::new(
				ValueParam::new(Value::Float(1.0)),
				ValueParam::new(Value::Float(0.0)),
			)),
			OperatorTemplate::ValueCompareGT => Box::new(ValueCompareGTOp::new(
				ValueParam::new(Value::Float(0.0)),
				ValueParam::new(Value::Float(0.0)),
			)),
			OperatorTemplate::ValueSelect => Box::new(ValueSelectOp::new(
				ValueParam::new(Value::Bool(false)),
				ValueParam::new(Value::Float(0.0)),
				ValueParam::new(Value::Float(1.0)),
			)),
			OperatorTemplate::ValueDelay1 => {
				Box::new(ValueDelay1Op::new(ValueParam::new(Value::Float(0.0))))
			}
			OperatorTemplate::ImageSolidColor => Box::new(ImageSolidColorOp::new(
				ValueParam::new(Value::Vec4([0.2, 0.2, 0.2, 1.0])),
				256,
				256,
			)),
			OperatorTemplate::ImageMix => {
				let (a, b) = self.default_image_pair();
				Box::new(ImageMixOp::new(a, b, ValueParam::new(Value::Float(0.5))))
			}
			OperatorTemplate::ImageBlur1D => {
				let input = self.ensure_image_node();
				Box::new(ImageBlur1DOp::new(
					input,
					ValueParam::new(Value::Float(4.0)),
				))
			}
			OperatorTemplate::ImageDelay1 => {
				let input = self.ensure_image_node();
				Box::new(ImageDelay1Op::new(input))
			}
		}
	}

	fn default_image_pair(&mut self) -> (NodeId, NodeId) {
		let images = self.image_nodes();
		match images.as_slice() {
			[a, b, ..] => (a.id, b.id),
			[a] => {
				let second = self.add_auto_node(OperatorTemplate::ImageSolidColor);
				(a.id, second)
			}
			_ => {
				let first = self.add_auto_node(OperatorTemplate::ImageSolidColor);
				let second = self.add_auto_node(OperatorTemplate::ImageSolidColor);
				(first, second)
			}
		}
	}

	fn ensure_image_node(&mut self) -> NodeId {
		if let Some(first) = self.image_nodes().first() {
			first.id
		} else {
			self.add_auto_node(OperatorTemplate::ImageSolidColor)
		}
	}

	fn value_nodes(&mut self) -> Vec<ValueNodeInfo> {
		let ctx = EvalContext { fps: self.fps };

		self.nodes
			.iter()
			.filter_map(|entry| {
				let (is_value, kind) = match self.graph.node(entry.id) {
					Some(node) => (
						node.operator.output_type() == OutputType::Value,
						node.operator.value_kind(),
					),
					None => (false, None),
				};

				if !is_value {
					return None;
				}

				let preview = self.graph.eval_value(entry.id, self.time, &ctx).ok();
				Some(ValueNodeInfo {
					id: entry.id,
					name: entry.name.clone(),
					kind,
					preview,
				})
			})
			.collect()
	}

	fn image_nodes(&self) -> Vec<ImageNodeInfo> {
		self.nodes
			.iter()
			.filter_map(|entry| {
				let node = self.graph.node(entry.id)?;
				if node.operator.output_type() != OutputType::Image {
					return None;
				}
				Some(ImageNodeInfo {
					id: entry.id,
					name: entry.name.clone(),
				})
			})
			.collect()
	}

	fn evaluate_selected(&mut self, ctx: &egui::Context) {
		let Some(node_id) = self.selected else {
			self.last_error = Some("No node selected".to_string());
			self.last_eval = None;
			return;
		};

		let output_type = match self.graph.node(node_id) {
			Some(node) => node.operator.output_type(),
			None => {
				self.last_error = Some("Selected node missing".to_string());
				self.last_eval = None;
				return;
			}
		};

		let ctx_eval = EvalContext { fps: self.fps };
		match output_type {
			OutputType::Value => match self.graph.eval_value(node_id, self.time, &ctx_eval) {
				Ok(value) => {
					self.last_eval = Some(EvalOutput::Value(value));
					self.last_error = None;
				}
				Err(err) => {
					self.last_error = Some(err.to_string());
					self.last_eval = None;
				}
			},
			OutputType::Image => match self.graph.eval_image(node_id, self.time, &ctx_eval) {
				Ok(image) => {
					let color_image = image_to_color_image(&image);
					let size = [image.width as usize, image.height as usize];
					match &mut self.preview {
						Some(preview) if preview.node_id == node_id => {
							preview.texture.set(color_image, TextureOptions::default());
							preview.size = size;
						}
						_ => {
							let texture = ctx.load_texture(
								format!("node-image-{}", node_id.value()),
								color_image,
								TextureOptions::default(),
							);
							self.preview = Some(EvalPreview {
								node_id,
								texture,
								size,
							});
						}
					}
					self.last_eval = Some(EvalOutput::Image(size));
					self.last_error = None;
				}
				Err(err) => {
					self.last_error = Some(err.to_string());
					self.last_eval = None;
				}
			},
		}
		self.eval_dirty = false;
	}

	fn step_time(&mut self) {
		let step = 1.0 / self.fps.max(1.0);
		self.time += step;
		self.eval_dirty = true;
	}

	fn ui_toolbar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
		ui.horizontal(|ui| {
			let play_label = if self.playing { "Pause" } else { "Play" };
			if ui.button(play_label).clicked() {
				self.playing = !self.playing;
			}
			if ui.button("Step").clicked() {
				self.step_time();
			}

			ui.separator();

			let time_changed = ui
				.add(egui::DragValue::new(&mut self.time).speed(0.01))
				.changed();
			ui.label("Time");
			if time_changed {
				self.time = self.time.max(0.0);
				self.eval_dirty = true;
			}

			let fps_changed = ui
				.add(egui::DragValue::new(&mut self.fps).speed(1.0))
				.changed();
			ui.label("FPS");
			if fps_changed {
				self.fps = self.fps.clamp(1.0, 240.0);
				self.eval_dirty = true;
			}

			ui.separator();

			if ui.button("Evaluate").clicked() {
				self.evaluate_selected(ctx);
			}
			if ui.button("Clear Cache").clicked() {
				self.graph.clear_cache();
				self.eval_dirty = true;
			}

			ui.checkbox(&mut self.auto_eval, "Auto Eval");
		});
	}

	fn ui_node_list(&mut self, ui: &mut egui::Ui) {
		ui.heading("Nodes");
		ui.horizontal(|ui| {
			ui.label("Operator");
			let _ = eguis_combo_operator(ui, "add-operator", &mut self.operator_to_add);
			if ui.button("Add").clicked() {
				self.add_node_with_operator(self.operator_to_add);
			}
		});

		ui.separator();

		let mut to_remove = Vec::new();
		for entry in &self.nodes {
			let output_type = self
				.graph
				.node(entry.id)
				.map(|node| node.operator.output_type())
				.unwrap_or(OutputType::Value);
			let selected = self.selected == Some(entry.id);
			ui.horizontal(|ui| {
				let label = format!("{} (#{} )", entry.name, entry.id.value());
				if ui.selectable_label(selected, label).clicked() {
					self.selected = Some(entry.id);
					self.eval_dirty = true;
				}
				ui.label(match output_type {
					OutputType::Value => "V",
					OutputType::Image => "I",
				});
				if ui.small_button("x").clicked() {
					to_remove.push(entry.id);
				}
			});
		}

		for node_id in to_remove {
			self.remove_node(node_id);
		}
	}

	fn ui_inspector(&mut self, ui: &mut egui::Ui) {
		ui.heading("Inspector");
		let Some(selected) = self.selected else {
			ui.label("No node selected");
			return;
		};

		{
			if let Some(entry) = self.nodes.iter_mut().find(|node| node.id == selected) {
				ui.horizontal(|ui| {
					ui.label("Name");
					ui.text_edit_singleline(&mut entry.name);
				});
			}
		}

		let current_template = match self.graph.node(selected) {
			Some(node) => OperatorTemplate::from_operator(node.operator.as_ref()),
			None => {
				ui.label("Selected node missing");
				return;
			}
		};

		ui.label(format!("Node ID: {}", selected.value()));
		let output_type = self
			.graph
			.node(selected)
			.map(|node| node.operator.output_type())
			.unwrap_or(OutputType::Value);
		ui.label(format!("Output: {:?}", output_type));

		if current_template.is_none() {
			ui.label("Custom operator");
		}

		let mut template = current_template.unwrap_or(OperatorTemplate::ValueConst);
		let template_changed =
			eguis_combo_operator(ui, ("operator", selected.value()), &mut template);
		let reset_clicked = ui.button("Reset Operator").clicked();

		if template_changed {
			self.apply_operator_change(selected, template);
		}

		if reset_clicked {
			let reset = match output_type {
				OutputType::Value => OperatorTemplate::ValueConst,
				OutputType::Image => OperatorTemplate::ImageSolidColor,
			};
			self.apply_operator_change(selected, reset);
		}

		let value_nodes = self.value_nodes();
		let image_nodes = self.image_nodes();
		let mut params_changed = false;

		if let Some(node) = self.graph.node_mut(selected) {
			ui.separator();

			let params = node.operator.parameters();
			if params.is_empty() {
				ui.label("No parameters");
			}

			for (index, param) in params.into_iter().enumerate() {
				let label = param.label;
				let mut changed = false;

				match (param.kind, param.value) {
					(
						ParamKind::ValueParam {
							allow_mapping,
							ui: ValueParamUi::Float,
							..
						},
						ParamValue::ValueParam(mut value_param),
					) => {
						changed |= float_param_editor(
							ui,
							label.as_str(),
							&mut value_param,
							&value_nodes,
							selected,
							allow_mapping,
						);
						if changed {
							node.operator
								.set_parameter(index, ParamValue::ValueParam(value_param));
						}
					}
					(
						ParamKind::ValueParam {
							ui: ValueParamUi::Bool,
							..
						},
						ParamValue::ValueParam(mut value_param),
					) => {
						changed |= bool_param_editor(ui, label.as_str(), &mut value_param);
						if changed {
							node.operator
								.set_parameter(index, ParamValue::ValueParam(value_param));
						}
					}
					(
						ParamKind::ValueParam {
							allow_mapping,
							ui: ValueParamUi::Color,
							..
						},
						ParamValue::ValueParam(mut value_param),
					) => {
						changed |= color_param_editor(
							ui,
							label.as_str(),
							&mut value_param,
							&value_nodes,
							selected,
							allow_mapping,
						);
						if changed {
							node.operator
								.set_parameter(index, ParamValue::ValueParam(value_param));
						}
					}
					(
						ParamKind::ValueParam { allow_mapping, .. },
						ParamValue::ValueParam(mut value_param),
					) => {
						changed |= value_param_editor(
							ui,
							label.as_str(),
							&mut value_param,
							&value_nodes,
							selected,
							allow_mapping,
						);
						if changed {
							node.operator
								.set_parameter(index, ParamValue::ValueParam(value_param));
						}
					}
					(ParamKind::U32 { min, max }, ParamValue::U32(mut value)) => {
						changed |= u32_param_editor(ui, label.as_str(), &mut value, min, max);
						if changed {
							node.operator.set_parameter(index, ParamValue::U32(value));
						}
					}
					(ParamKind::ImageInput, ParamValue::ImageInput(mut input)) => {
						changed |= image_input_editor(
							ui,
							label.as_str(),
							&mut input,
							&image_nodes,
							selected,
						);
						if changed {
							node.operator
								.set_parameter(index, ParamValue::ImageInput(input));
						}
					}
					_ => {
						ui.label(format!("{}: unsupported parameter", label));
					}
				}

				if changed {
					params_changed = true;
				}
			}

			ui.separator();
			ui.label(format!("State: {:?}", node.state));
		}

		if params_changed {
			self.graph.clear_cache();
			self.eval_dirty = true;
		}
	}

	fn ui_evaluation(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
		ui.heading("Evaluation");
		if ui.button("Evaluate Selected").clicked() {
			self.evaluate_selected(ctx);
		}

		if let Some(err) = &self.last_error {
			ui.colored_label(egui::Color32::from_rgb(200, 80, 80), err);
		}

		if let Some(output) = &self.last_eval {
			match output {
				EvalOutput::Value(value) => {
					ui.label(format!("Value: {}", format_value(value)));
				}
				EvalOutput::Image(size) => {
					ui.label(format!("Image: {}x{}", size[0], size[1]));
				}
			}
		} else {
			ui.label("No evaluation yet");
		}

		ui.separator();
		ui.label("Preview");

		let preview_size = {
			let available = ui.available_size();
			let width = available.x.max(200.0);
			let height = available.y.clamp(180.0, 320.0);
			egui::vec2(width, height)
		};

		let preview_ready = self
			.preview
			.as_ref()
			.filter(|preview| self.selected == Some(preview.node_id));

		egui::Frame::default()
			.fill(egui::Color32::from_gray(20))
			.corner_radius(4.0)
			.inner_margin(egui::Margin::same(6))
			.show(ui, |ui| {
				ui.set_min_size(preview_size);
				if let Some(preview) = preview_ready {
					let available = ui.available_size();
					let size = preview.texture.size_vec2();
					let scale = (available.x / size.x).min(available.y / size.y).min(1.0);
					let draw_size = size * scale;
					ui.centered_and_justified(|ui| {
						ui.add(
							egui::Image::new((preview.texture.id(), size))
								.fit_to_exact_size(draw_size),
						);
					});
				} else {
					ui.centered_and_justified(|ui| {
						ui.label("No image preview");
					});
				}
			});
	}
}

impl eframe::App for PlaygroundApp {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		if self.playing {
			let dt = ctx.input(|i| i.unstable_dt) as f64;
			self.time += dt;
			self.eval_dirty = true;
			ctx.request_repaint();
		}

		eguitop(ctx, |ui| self.ui_toolbar(ui, ctx));

		eguiside(ctx, |ui| self.ui_node_list(ui));

		eguiright(ctx, |ui| {
			egui::ScrollArea::vertical()
				.auto_shrink([false, false])
				.show(ui, |ui| self.ui_inspector(ui));
		});

		egui::CentralPanel::default().show(ctx, |ui| {
			self.ui_evaluation(ui, ctx);
		});

		if self.auto_eval && self.eval_dirty {
			self.evaluate_selected(ctx);
		}
	}
}

fn eguitop(ctx: &egui::Context, f: impl FnOnce(&mut egui::Ui)) {
	egui::TopBottomPanel::top("toolbar").show(ctx, f);
}

fn eguiside(ctx: &egui::Context, f: impl FnOnce(&mut egui::Ui)) {
	egui::SidePanel::left("nodes").resizable(true).show(ctx, f);
}

fn eguiright(ctx: &egui::Context, f: impl FnOnce(&mut egui::Ui)) {
	egui::SidePanel::right("inspector")
		.resizable(true)
		.default_width(360.0)
		.min_width(240.0)
		.max_width(520.0)
		.show(ctx, f);
}

fn eguis_combo_operator(
	ui: &mut egui::Ui,
	id_source: impl std::hash::Hash,
	selected: &mut OperatorTemplate,
) -> bool {
	let mut changed = false;
	egui::ComboBox::from_id_salt(id_source)
		.selected_text(selected.label())
		.show_ui(ui, |ui| {
			for template in OperatorTemplate::all() {
				if ui
					.selectable_value(selected, *template, template.label())
					.changed()
				{
					changed = true;
				}
			}
		});
	changed
}

fn value_kind_label(kind: ValueKind) -> &'static str {
	match kind {
		ValueKind::Float => "Float",
		ValueKind::Vec2 => "Vec2",
		ValueKind::Vec3 => "Vec3",
		ValueKind::Vec4 => "Vec4",
		ValueKind::Bool => "Bool",
	}
}

fn format_value(value: &Value) -> String {
	match value {
		Value::Float(v) => format!("{v:.3}"),
		Value::Vec2(v) => format!("[{:.3}, {:.3}]", v[0], v[1]),
		Value::Vec3(v) => format!("[{:.3}, {:.3}, {:.3}]", v[0], v[1], v[2]),
		Value::Vec4(v) => format!("[{:.3}, {:.3}, {:.3}, {:.3}]", v[0], v[1], v[2], v[3]),
		Value::Bool(v) => format!("{v}"),
	}
}

fn value_editor(ui: &mut egui::Ui, label: impl std::hash::Hash, value: &mut Value) -> bool {
	let mut changed = false;
	let mut kind = value.kind();
	egui::ComboBox::from_id_salt(label)
		.selected_text(value_kind_label(kind))
		.show_ui(ui, |ui| {
			for variant in [
				ValueKind::Float,
				ValueKind::Vec2,
				ValueKind::Vec3,
				ValueKind::Vec4,
				ValueKind::Bool,
			] {
				if ui
					.selectable_value(&mut kind, variant, value_kind_label(variant))
					.clicked()
				{
					changed = true;
				}
			}
		});

	if kind != value.kind() {
		*value = match kind {
			ValueKind::Float => Value::Float(0.0),
			ValueKind::Vec2 => Value::Vec2([0.0, 0.0]),
			ValueKind::Vec3 => Value::Vec3([0.0, 0.0, 0.0]),
			ValueKind::Vec4 => Value::Vec4([0.0, 0.0, 0.0, 1.0]),
			ValueKind::Bool => Value::Bool(false),
		};
		changed = true;
	}

	match value {
		Value::Float(v) => {
			changed |= ui.add(egui::DragValue::new(v).speed(0.05)).changed();
		}
		Value::Vec2(v) => {
			changed |= ui
				.add(egui::DragValue::new(&mut v[0]).speed(0.05))
				.changed();
			changed |= ui
				.add(egui::DragValue::new(&mut v[1]).speed(0.05))
				.changed();
		}
		Value::Vec3(v) => {
			changed |= ui
				.add(egui::DragValue::new(&mut v[0]).speed(0.05))
				.changed();
			changed |= ui
				.add(egui::DragValue::new(&mut v[1]).speed(0.05))
				.changed();
			changed |= ui
				.add(egui::DragValue::new(&mut v[2]).speed(0.05))
				.changed();
		}
		Value::Vec4(v) => {
			changed |= ui
				.add(egui::DragValue::new(&mut v[0]).speed(0.05))
				.changed();
			changed |= ui
				.add(egui::DragValue::new(&mut v[1]).speed(0.05))
				.changed();
			changed |= ui
				.add(egui::DragValue::new(&mut v[2]).speed(0.05))
				.changed();
			changed |= ui
				.add(egui::DragValue::new(&mut v[3]).speed(0.05))
				.changed();
		}
		Value::Bool(v) => {
			changed |= ui.checkbox(v, "").changed();
		}
	}

	changed
}

fn value_param_editor(
	ui: &mut egui::Ui,
	label: &str,
	param: &mut ValueParam,
	value_nodes: &[ValueNodeInfo],
	current_node: NodeId,
	allow_mapping: bool,
) -> bool {
	let mut changed = false;
	ui.group(|ui| {
		ui.label(label);
		changed |= value_editor(ui, (label, "value"), &mut param.base);

		let mapping_allowed = allow_mapping && param.base.kind() != ValueKind::Bool;
		let mut use_mapping = param.mapping.is_some();

		if mapping_allowed {
			if ui.checkbox(&mut use_mapping, "Map").changed() {
				if use_mapping {
					param.mapping = value_nodes
						.iter()
						.find(|node| node.id != current_node)
						.map(|node| node.id);
				} else {
					param.mapping = None;
				}
				changed = true;
			}
		} else if param.mapping.is_some() {
			param.mapping = None;
			changed = true;
		}

		if use_mapping {
			let mut selected = param.mapping;
			egui::ComboBox::from_id_salt((label, "map"))
				.selected_text(value_node_selected_label(selected, value_nodes))
				.show_ui(ui, |ui| {
					ui.selectable_value(&mut selected, None, "None");
					for node in value_nodes {
						if node.id == current_node {
							continue;
						}
						ui.selectable_value(&mut selected, Some(node.id), node_label(node));
					}
				});

			if selected != param.mapping {
				param.mapping = selected;
				changed = true;
			}
		}
	});
	changed
}

fn float_param_editor(
	ui: &mut egui::Ui,
	label: &str,
	param: &mut ValueParam,
	value_nodes: &[ValueNodeInfo],
	current_node: NodeId,
	allow_mapping: bool,
) -> bool {
	if !matches!(param.base, Value::Float(_)) {
		param.base = Value::Float(0.0);
		param.mapping = None;
	}

	let mut changed = false;
	ui.group(|ui| {
		ui.label(label);
		if let Value::Float(value) = &mut param.base {
			changed |= ui.add(egui::DragValue::new(value).speed(0.05)).changed();
		}

		let mut use_mapping = param.mapping.is_some();
		if allow_mapping {
			if ui.checkbox(&mut use_mapping, "Map").changed() {
				if use_mapping {
					param.mapping = value_nodes
						.iter()
						.find(|node| node.id != current_node)
						.map(|node| node.id);
				} else {
					param.mapping = None;
				}
				changed = true;
			}

			if use_mapping {
				let mut selected = param.mapping;
				egui::ComboBox::from_id_salt((label, "map"))
					.selected_text(value_node_selected_label(selected, value_nodes))
					.show_ui(ui, |ui| {
						ui.selectable_value(&mut selected, None, "None");
						for node in value_nodes {
							if node.id == current_node {
								continue;
							}
							if node.kind == Some(ValueKind::Float) || node.kind.is_none() {
								ui.selectable_value(&mut selected, Some(node.id), node_label(node));
							}
						}
					});

				if selected != param.mapping {
					param.mapping = selected;
					changed = true;
				}
			}
		} else if param.mapping.is_some() {
			param.mapping = None;
			changed = true;
		}
	});
	changed
}

fn bool_param_editor(ui: &mut egui::Ui, label: &str, param: &mut ValueParam) -> bool {
	if !matches!(param.base, Value::Bool(_)) {
		param.base = Value::Bool(false);
		param.mapping = None;
	}

	let mut changed = false;
	ui.group(|ui| {
		ui.label(label);
		if let Value::Bool(value) = &mut param.base {
			changed |= ui.checkbox(value, "").changed();
		}
	});
	changed
}

fn color_param_editor(
	ui: &mut egui::Ui,
	label: &str,
	param: &mut ValueParam,
	value_nodes: &[ValueNodeInfo],
	current_node: NodeId,
	allow_mapping: bool,
) -> bool {
	if !matches!(param.base, Value::Vec4(_)) {
		param.base = Value::Vec4([0.0, 0.0, 0.0, 1.0]);
		param.mapping = None;
	}

	let mut changed = false;
	ui.group(|ui| {
		ui.label(label);
		if let Value::Vec4(color) = &mut param.base {
			let mut rgba = *color;
			if ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed() {
				*color = rgba;
				changed = true;
			}
		}

		let mut use_mapping = param.mapping.is_some();
		if allow_mapping {
			if ui.checkbox(&mut use_mapping, "Map").changed() {
				if use_mapping {
					param.mapping = value_nodes
						.iter()
						.find(|node| node.id != current_node)
						.map(|node| node.id);
				} else {
					param.mapping = None;
				}
				changed = true;
			}

			if use_mapping {
				let mut selected = param.mapping;
				egui::ComboBox::from_id_salt((label, "map"))
					.selected_text(value_node_selected_label(selected, value_nodes))
					.show_ui(ui, |ui| {
						ui.selectable_value(&mut selected, None, "None");
						for node in value_nodes {
							if node.id == current_node {
								continue;
							}
							if node.kind == Some(ValueKind::Vec4) || node.kind.is_none() {
								ui.selectable_value(&mut selected, Some(node.id), node_label(node));
							}
						}
					});

				if selected != param.mapping {
					param.mapping = selected;
					changed = true;
				}
			}
		} else if param.mapping.is_some() {
			param.mapping = None;
			changed = true;
		}
	});
	changed
}

fn image_input_editor(
	ui: &mut egui::Ui,
	label: &str,
	input: &mut NodeId,
	image_nodes: &[ImageNodeInfo],
	current_node: NodeId,
) -> bool {
	let mut changed = false;
	ui.group(|ui| {
		ui.label(label);
		let mut selected = Some(*input);
		egui::ComboBox::from_id_salt((label, "input"))
			.selected_text(selected.map_or("None".to_string(), |id| format!("#{}", id.value())))
			.show_ui(ui, |ui| {
				for node in image_nodes {
					if node.id == current_node {
						continue;
					}
					ui.selectable_value(&mut selected, Some(node.id), node.name.clone());
				}
			});

		if let Some(node_id) = selected
			&& node_id != *input
		{
			*input = node_id;
			changed = true;
		}
	});
	changed
}

fn u32_param_editor(ui: &mut egui::Ui, label: &str, value: &mut u32, min: u32, max: u32) -> bool {
	let mut changed = false;
	ui.group(|ui| {
		ui.label(label);
		let response = ui.add(egui::DragValue::new(value).speed(1.0));
		if response.changed() {
			*value = (*value).clamp(min, max);
			changed = true;
		}
	});
	changed
}

fn node_label(node: &ValueNodeInfo) -> String {
	let mut label = match node.kind {
		Some(kind) => format!("{} ({})", node.name, value_kind_label(kind)),
		None => node.name.clone(),
	};

	match &node.preview {
		Some(value) => {
			label.push_str(" = ");
			label.push_str(&format_value(value));
		}
		None => {
			label.push_str(" = --");
		}
	}

	label
}

fn value_node_label_for_id(id: NodeId, value_nodes: &[ValueNodeInfo]) -> String {
	value_nodes
		.iter()
		.find(|node| node.id == id)
		.map(node_label)
		.unwrap_or_else(|| format!("#{}", id.value()))
}

fn value_node_selected_label(selected: Option<NodeId>, value_nodes: &[ValueNodeInfo]) -> String {
	selected
		.map(|id| value_node_label_for_id(id, value_nodes))
		.unwrap_or_else(|| "None".to_string())
}

fn image_to_color_image(image: &Image) -> ColorImage {
	let size = [image.width as usize, image.height as usize];
	let mut pixels = Vec::with_capacity(size[0] * size[1]);
	for idx in (0..image.rgba.len()).step_by(4) {
		let r = (image.rgba[idx].clamp(0.0, 1.0) * 255.0).round() as u8;
		let g = (image.rgba[idx + 1].clamp(0.0, 1.0) * 255.0).round() as u8;
		let b = (image.rgba[idx + 2].clamp(0.0, 1.0) * 255.0).round() as u8;
		let a = (image.rgba[idx + 3].clamp(0.0, 1.0) * 255.0).round() as u8;
		pixels.push(egui::Color32::from_rgba_unmultiplied(r, g, b, a));
	}

	ColorImage::new(size, pixels)
}

fn main() -> eframe::Result<()> {
	env_logger::init();

	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_inner_size([1280.0, 720.0])
			.with_title("Graph Playground"),
		..Default::default()
	};

	eframe::run_native(
		"Graph Playground",
		options,
		Box::new(|_cc| Ok(Box::new(PlaygroundApp::new()))),
	)
}
