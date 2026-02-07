use crate::message::{AppPanelMessage, GraphMessage, InspectorMessage, PropertyMessage};
use crate::panel_content::PanelContent;
use crate::theme::panel::INSPECTOR_BG;
use crate::theme::{TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY};
use crate::widgets::draggable_number::draggable_number;
use crate::widgets::video_view::VideoView;
use iced::widget::{Space, button, checkbox, column, container, row, shader, text};
use iced::{Alignment, Element, Length};
use nade_core::{EvalContext, FrameData, Graph, Image, NodeId, Transform, Value, ValueParam};
use operators::{
	ImageBlur1DOp, ImageDelay1Op, ImageMixOp, ImageSolidColorOp, ValueAddOp, ValueConstOp,
	ValueDelay1Op, ValueMulOp, ValueSinOp,
};
use panel_system::PanelSystemMessage;

#[derive(Debug)]
pub struct InspectorUiState {
	graph: Graph,
	ids: GraphIds,
	transform_ids: Option<TransformOperatorIds>,
	pub freq: f32,
	pub phase: f32,
	pub blur_base: f32,
	pub blur_scale: f32,
	pub color_a: [f32; 4],
	pub color_b: [f32; 4],
	pub eval_fps: f32,
	pub use_value_delay: bool,
	pub use_image_delay: bool,
	pub last_lfo: Option<f32>,
	pub last_blur_radius: Option<f32>,
	pub last_frame: Option<FrameData>,
	pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct GraphIds {
	freq: NodeId,
	phase: NodeId,
	blur_base: NodeId,
	blur_scale: NodeId,
	color_a: NodeId,
	color_b: NodeId,
	lfo: NodeId,
	lfo_delay: NodeId,
	blur_radius: NodeId,
	blur: NodeId,
	image_delay: NodeId,
}

#[derive(Debug, Clone, Copy)]
struct TransformOperatorIds {
	position: NodeId,
	rotation: NodeId,
	scale: NodeId,
	opacity: NodeId,
}

#[derive(Debug, Clone, Copy)]
struct SampleGraphConfig {
	freq: f32,
	phase: f32,
	blur_base: f32,
	blur_scale: f32,
	color_a: [f32; 4],
	color_b: [f32; 4],
	width: u32,
	height: u32,
}

impl InspectorUiState {
	pub fn new() -> Self {
		let freq = 0.5;
		let phase = 0.0;
		let blur_base = 1.0;
		let blur_scale = 10.0;
		let color_a = [1.0, 0.2, 0.2, 1.0];
		let color_b = [0.2, 0.4, 1.0, 1.0];
		let eval_fps = 60.0;

		let (graph, ids) = build_sample_graph(SampleGraphConfig {
			freq,
			phase,
			blur_base,
			blur_scale,
			color_a,
			color_b,
			width: 256,
			height: 256,
		});

		let mut state = Self {
			graph,
			ids,
			transform_ids: None,
			freq,
			phase,
			blur_base,
			blur_scale,
			color_a,
			color_b,
			eval_fps,
			use_value_delay: false,
			use_image_delay: false,
			last_lfo: None,
			last_blur_radius: None,
			last_frame: None,
			last_error: None,
		};

		state.evaluate(0.0);
		state
	}

	pub fn update(
		&mut self,
		msg: InspectorMessage,
		time: f64,
		current_selection: Option<Transform>,
	) -> Option<Transform> {
		match msg {
			InspectorMessage::AttachTransformOperator => {
				self.attach_transform_operator(current_selection.unwrap_or_default());
				self.evaluate(time);
				self.transform()
			}
			InspectorMessage::Property(prop_msg) => {
				if self.transform_ids.is_none() {
					self.attach_transform_operator(current_selection.unwrap_or_default());
				}
				let updated = self.update_transform_operator(prop_msg);
				self.evaluate(time);
				updated
			}
			InspectorMessage::Graph(graph_msg) => {
				self.update_graph(graph_msg, time);
				None
			}
		}
	}

	pub fn has_transform_operator(&self) -> bool {
		self.transform_ids.is_some()
	}

	pub fn transform(&self) -> Option<Transform> {
		let ids = self.transform_ids?;
		self.read_transform(ids)
	}

	fn update_graph(&mut self, msg: GraphMessage, time: f64) {
		match msg {
			GraphMessage::FreqChanged(value) => {
				self.freq = value.max(0.0);
				self.set_value_const(self.ids.freq, Value::Float(self.freq));
			}
			GraphMessage::PhaseChanged(value) => {
				self.phase = value;
				self.set_value_const(self.ids.phase, Value::Float(self.phase));
			}
			GraphMessage::BlurBaseChanged(value) => {
				self.blur_base = value.max(0.0);
				self.set_value_const(self.ids.blur_base, Value::Float(self.blur_base));
			}
			GraphMessage::BlurScaleChanged(value) => {
				self.blur_scale = value.max(0.0);
				self.set_value_const(self.ids.blur_scale, Value::Float(self.blur_scale));
			}
			GraphMessage::ColorAChanged(channel, value) => {
				if let Some(slot) = self.color_a.get_mut(channel) {
					*slot = value.clamp(0.0, 1.0);
					self.set_value_const(self.ids.color_a, Value::Vec4(self.color_a));
				}
			}
			GraphMessage::ColorBChanged(channel, value) => {
				if let Some(slot) = self.color_b.get_mut(channel) {
					*slot = value.clamp(0.0, 1.0);
					self.set_value_const(self.ids.color_b, Value::Vec4(self.color_b));
				}
			}
			GraphMessage::EvalFpsChanged(value) => {
				self.eval_fps = value.max(1.0);
				self.graph.clear_cache();
			}
			GraphMessage::UseValueDelay(enabled) => {
				self.use_value_delay = enabled;
			}
			GraphMessage::UseImageDelay(enabled) => {
				self.use_image_delay = enabled;
			}
			GraphMessage::Evaluate => {}
		}

		self.evaluate(time);
	}

	fn attach_transform_operator(&mut self, initial: Transform) {
		if self.transform_ids.is_some() {
			return;
		}

		let position = self
			.graph
			.add_node(ValueConstOp::new(Value::Vec3(initial.position)));
		let rotation = self
			.graph
			.add_node(ValueConstOp::new(Value::Vec3(initial.rotation)));
		let scale = self
			.graph
			.add_node(ValueConstOp::new(Value::Vec3(initial.scale)));
		let opacity = self
			.graph
			.add_node(ValueConstOp::new(Value::Float(initial.opacity)));

		self.transform_ids = Some(TransformOperatorIds {
			position,
			rotation,
			scale,
			opacity,
		});
		self.graph.clear_cache();
	}

	fn update_transform_operator(&mut self, msg: PropertyMessage) -> Option<Transform> {
		let ids = self.transform_ids?;
		let mut transform = self.read_transform(ids)?;

		match msg {
			PropertyMessage::Position(axis, val) => {
				if axis < 3 {
					transform.position[axis] = val;
				}
			}
			PropertyMessage::Rotation(axis, val) => {
				if axis < 3 {
					transform.rotation[axis] = val;
				}
			}
			PropertyMessage::Scale(axis, val) => {
				if axis < 3 {
					transform.scale[axis] = val;
				}
			}
			PropertyMessage::Opacity(val) => {
				transform.opacity = val;
			}
		}

		self.set_value_const(ids.position, Value::Vec3(transform.position));
		self.set_value_const(ids.rotation, Value::Vec3(transform.rotation));
		self.set_value_const(ids.scale, Value::Vec3(transform.scale));
		self.set_value_const(ids.opacity, Value::Float(transform.opacity));

		Some(transform)
	}

	fn read_transform(&self, ids: TransformOperatorIds) -> Option<Transform> {
		let position = match self.const_value(ids.position)? {
			Value::Vec3(value) => value,
			_ => return None,
		};
		let rotation = match self.const_value(ids.rotation)? {
			Value::Vec3(value) => value,
			_ => return None,
		};
		let scale = match self.const_value(ids.scale)? {
			Value::Vec3(value) => value,
			_ => return None,
		};
		let opacity = match self.const_value(ids.opacity)? {
			Value::Float(value) => value,
			_ => return None,
		};

		Some(Transform {
			position,
			rotation,
			scale,
			opacity,
		})
	}

	pub fn evaluate(&mut self, time: f64) {
		let ctx = EvalContext {
			fps: f64::from(self.eval_fps),
		};
		let lfo_node = if self.use_value_delay {
			self.ids.lfo_delay
		} else {
			self.ids.lfo
		};
		let image_node = if self.use_image_delay {
			self.ids.image_delay
		} else {
			self.ids.blur
		};

		let lfo_value = match self.graph.eval_value(lfo_node, time, &ctx) {
			Ok(value) => match value {
				Value::Float(v) => v,
				other => {
					self.last_error = Some(format!("LFO expected Float, got {:?}", other.kind()));
					return;
				}
			},
			Err(err) => {
				self.last_error = Some(err.to_string());
				return;
			}
		};

		let blur_radius = match self.graph.eval_value(self.ids.blur_radius, time, &ctx) {
			Ok(value) => match value {
				Value::Float(v) => v,
				other => {
					self.last_error = Some(format!(
						"Blur radius expected Float, got {:?}",
						other.kind()
					));
					return;
				}
			},
			Err(err) => {
				self.last_error = Some(err.to_string());
				return;
			}
		};

		let image = match self.graph.eval_image(image_node, time, &ctx) {
			Ok(image) => image,
			Err(err) => {
				self.last_error = Some(err.to_string());
				return;
			}
		};

		self.last_lfo = Some(lfo_value);
		self.last_blur_radius = Some(blur_radius);
		self.last_frame = Some(image_to_frame(&image));
		self.last_error = None;
	}

	fn set_value_const(&mut self, node_id: NodeId, value: Value) {
		if let Some(node) = self.graph.node_mut(node_id)
			&& let Some(op) = node.operator.as_any_mut().downcast_mut::<ValueConstOp>()
		{
			op.value = value;
			self.graph.clear_cache();
		}
	}

	fn const_value(&self, node_id: NodeId) -> Option<Value> {
		let node = self.graph.node(node_id)?;
		let op = node.operator.as_any().downcast_ref::<ValueConstOp>()?;
		Some(op.value)
	}
}

pub fn view<'a>(
	state: &'a InspectorUiState,
	preview_time: f32,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	let header = row![
		text("Inspector").size(14).color(TEXT_PRIMARY),
		text(format!("Time {:.2}s", preview_time))
			.size(12)
			.color(TEXT_SECONDARY)
	]
	.spacing(10)
	.align_y(Alignment::Center);

	let fps_row = row![
		text("Eval FPS").size(12).color(TEXT_MUTED),
		draggable_number(state.eval_fps, 60.0, |value| {
			InspectorMessage::Graph(GraphMessage::EvalFpsChanged(value))
		})
		.step(1.0)
	]
	.spacing(10)
	.align_y(Alignment::Center);

	let lfo_section = column![
		section_title("LFO"),
		param_row(
			"Freq (Hz)",
			state.freq,
			|value| { InspectorMessage::Graph(GraphMessage::FreqChanged(value)) },
			0.01
		),
		param_row(
			"Phase",
			state.phase,
			|value| { InspectorMessage::Graph(GraphMessage::PhaseChanged(value)) },
			0.01
		),
		value_row("LFO", state.last_lfo),
	]
	.spacing(6);

	let blur_section = column![
		section_title("Blur"),
		param_row(
			"Base Radius",
			state.blur_base,
			|value| { InspectorMessage::Graph(GraphMessage::BlurBaseChanged(value)) },
			0.1
		),
		param_row(
			"Scale",
			state.blur_scale,
			|value| { InspectorMessage::Graph(GraphMessage::BlurScaleChanged(value)) },
			0.1
		),
		value_row("Radius", state.last_blur_radius),
	]
	.spacing(6);

	let color_section = column![
		section_title("Colors"),
		color_row("Color A", state.color_a, |channel, value| {
			InspectorMessage::Graph(GraphMessage::ColorAChanged(channel, value))
		}),
		color_row("Color B", state.color_b, |channel, value| {
			InspectorMessage::Graph(GraphMessage::ColorBChanged(channel, value))
		}),
	]
	.spacing(6);

	let delay_row = row![
		checkbox(state.use_value_delay)
			.label("Value Delay1")
			.on_toggle(|enabled| { InspectorMessage::Graph(GraphMessage::UseValueDelay(enabled)) }),
		checkbox(state.use_image_delay)
			.label("Image Delay1")
			.on_toggle(|enabled| { InspectorMessage::Graph(GraphMessage::UseImageDelay(enabled)) }),
	]
	.spacing(10);

	let evaluate_button =
		button(text("Evaluate").size(12)).on_press(InspectorMessage::Graph(GraphMessage::Evaluate));

	let error_text = if let Some(err) = &state.last_error {
		text(err)
			.size(12)
			.color(iced::Color::from_rgb(0.9, 0.4, 0.4))
	} else {
		text("OK")
			.size(12)
			.color(iced::Color::from_rgb(0.4, 0.9, 0.6))
	};

	let preview: Element<'_, InspectorMessage> = if let Some(frame) = &state.last_frame {
		shader(VideoView::new(Some(frame.clone())))
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	} else {
		container(text("No Image").color(TEXT_MUTED))
			.width(Length::Fill)
			.height(Length::Fill)
			.center_x(Length::Fill)
			.center_y(Length::Fill)
			.into()
	};

	let content = column![
		header,
		transform_section(state),
		divider(),
		fps_row,
		divider(),
		lfo_section,
		blur_section,
		color_section,
		delay_row,
		evaluate_button,
		error_text,
		divider(),
		container(preview)
			.width(Length::Fill)
			.height(Length::Fill)
			.style(|_| container::Style {
				background: Some(iced::Color::from_rgb(0.08, 0.08, 0.08).into()),
				..Default::default()
			}),
	]
	.spacing(8)
	.padding(10);

	let panel: Element<'_, InspectorMessage> = container(content)
		.width(Length::Fill)
		.height(Length::Fill)
		.style(|_| container::Style {
			background: Some(INSPECTOR_BG.into()),
			..Default::default()
		})
		.into();

	panel.map(|msg| PanelSystemMessage::AppMessage(AppPanelMessage::Inspector(msg)))
}

fn transform_section<'a>(state: &'a InspectorUiState) -> Element<'a, InspectorMessage> {
	if let Some(transform) = state.transform() {
		column![
			row![
				section_title("Transform"),
				text("Attached")
					.size(11)
					.color(iced::Color::from_rgb(0.4, 0.9, 0.6))
			]
			.spacing(8)
			.align_y(Alignment::Center),
			transform_row("Position", &transform.position, |axis, value| {
				InspectorMessage::Property(PropertyMessage::Position(axis, value))
			}),
			transform_row("Rotation", &transform.rotation, |axis, value| {
				InspectorMessage::Property(PropertyMessage::Rotation(axis, value))
			}),
			transform_row("Scale", &transform.scale, |axis, value| {
				InspectorMessage::Property(PropertyMessage::Scale(axis, value))
			}),
			column![
				text("Opacity").size(12).color(TEXT_MUTED),
				draggable_number(transform.opacity, 1.0, |value| {
					InspectorMessage::Property(PropertyMessage::Opacity(value))
				})
				.step(0.01)
			]
			.spacing(5),
		]
		.spacing(6)
		.into()
	} else {
		let status_text = if state.has_transform_operator() {
			text("Transform operator is unavailable")
				.size(12)
				.color(TEXT_MUTED)
		} else {
			text("Transform operator is not attached")
				.size(12)
				.color(TEXT_MUTED)
		};

		column![
			section_title("Transform"),
			status_text,
			button(text("Attach Transform Operator").size(12))
				.on_press(InspectorMessage::AttachTransformOperator),
		]
		.spacing(6)
		.into()
	}
}

fn section_title<'a>(label: &'a str) -> iced::widget::Text<'a> {
	text(label).size(12).color(TEXT_PRIMARY)
}

fn param_row<'a>(
	label: &'a str,
	value: f32,
	on_change: impl Fn(f32) -> InspectorMessage + 'a,
	step: f32,
) -> Element<'a, InspectorMessage> {
	row![
		text(label).size(12).color(TEXT_MUTED).width(100),
		draggable_number(value, 0.0, on_change).step(step)
	]
	.spacing(10)
	.align_y(Alignment::Center)
	.into()
}

fn value_row<'a>(label: &'a str, value: Option<f32>) -> Element<'a, InspectorMessage> {
	let content = value
		.map(|v| format!("{v:.3}"))
		.unwrap_or_else(|| "--".to_string());
	row![
		text(label).size(12).color(TEXT_MUTED).width(100),
		text(content).size(12).color(TEXT_SECONDARY)
	]
	.spacing(10)
	.align_y(Alignment::Center)
	.into()
}

fn color_row<'a>(
	label: &'a str,
	color: [f32; 4],
	on_change: impl Fn(usize, f32) -> InspectorMessage + 'a + Clone,
) -> Element<'a, InspectorMessage> {
	let swatch = container(text(""))
		.width(Length::Fixed(18.0))
		.height(Length::Fixed(18.0))
		.style(move |_| container::Style {
			background: Some(iced::Color::from_rgba(color[0], color[1], color[2], color[3]).into()),
			..Default::default()
		});

	let channel = |index: usize, label: &'a str| {
		let on_change = on_change.clone();
		row![
			text(label).size(11).color(TEXT_MUTED),
			draggable_number(color[index], 0.0, move |v| on_change(index, v)).step(0.01)
		]
		.spacing(6)
	};

	column![
		row![text(label).size(12).color(TEXT_MUTED), swatch]
			.spacing(8)
			.align_y(Alignment::Center),
		row![
			channel(0, "R"),
			channel(1, "G"),
			channel(2, "B"),
			channel(3, "A")
		]
		.spacing(8)
		.align_y(Alignment::Center)
	]
	.spacing(4)
	.into()
}

fn transform_row<'a, F>(
	label: &'a str,
	values: &[f32; 3],
	message_fn: F,
) -> Element<'a, InspectorMessage>
where
	F: Fn(usize, f32) -> InspectorMessage + 'a + Clone,
{
	let axis_label = |axis: &'a str| text(axis).size(12).color(TEXT_MUTED).width(15);

	column![
		text(label).size(12).color(TEXT_MUTED),
		row![
			row![axis_label("X"), {
				let func = message_fn.clone();
				draggable_number(values[0], 0.0, move |v| func(0, v)).step(0.1)
			}]
			.spacing(5),
			row![axis_label("Y"), {
				let func = message_fn.clone();
				draggable_number(values[1], 0.0, move |v| func(1, v)).step(0.1)
			}]
			.spacing(5),
			row![axis_label("Z"), {
				let func = message_fn;
				draggable_number(values[2], 0.0, move |v| func(2, v)).step(0.1)
			}]
			.spacing(5),
		]
		.spacing(10)
	]
	.spacing(5)
	.into()
}

fn divider<'a>() -> Element<'a, InspectorMessage> {
	container(Space::new().width(Length::Fill).height(Length::Fixed(1.0)))
		.style(|_| container::Style {
			background: Some(iced::Color::from_rgb(0.2, 0.2, 0.2).into()),
			..Default::default()
		})
		.into()
}

fn build_sample_graph(config: SampleGraphConfig) -> (Graph, GraphIds) {
	let mut graph = Graph::new();

	let freq_node = graph.add_node(ValueConstOp::new(Value::Float(config.freq)));
	let phase_node = graph.add_node(ValueConstOp::new(Value::Float(config.phase)));
	let one_node = graph.add_node(ValueConstOp::new(Value::Float(1.0)));
	let half_node = graph.add_node(ValueConstOp::new(Value::Float(0.5)));

	let sin_node = graph.add_node(ValueSinOp::new(
		ValueParam::with_mapping(Value::Float(0.0), freq_node),
		ValueParam::with_mapping(Value::Float(0.0), phase_node),
	));

	let add_node = graph.add_node(ValueAddOp::new(
		ValueParam::with_mapping(Value::Float(0.0), sin_node),
		ValueParam::with_mapping(Value::Float(0.0), one_node),
	));

	let lfo_node = graph.add_node(ValueMulOp::new(
		ValueParam::with_mapping(Value::Float(0.0), add_node),
		ValueParam::with_mapping(Value::Float(0.0), half_node),
	));

	let lfo_delay_node = graph.add_node(ValueDelay1Op::new(ValueParam::with_mapping(
		Value::Float(0.0),
		lfo_node,
	)));

	let blur_base_node = graph.add_node(ValueConstOp::new(Value::Float(config.blur_base)));
	let blur_scale_node = graph.add_node(ValueConstOp::new(Value::Float(config.blur_scale)));
	let blur_scale_mul = graph.add_node(ValueMulOp::new(
		ValueParam::with_mapping(Value::Float(0.0), lfo_node),
		ValueParam::with_mapping(Value::Float(0.0), blur_scale_node),
	));
	let blur_radius_node = graph.add_node(ValueAddOp::new(
		ValueParam::with_mapping(Value::Float(0.0), blur_base_node),
		ValueParam::with_mapping(Value::Float(0.0), blur_scale_mul),
	));

	let color_a_node = graph.add_node(ValueConstOp::new(Value::Vec4(config.color_a)));
	let color_b_node = graph.add_node(ValueConstOp::new(Value::Vec4(config.color_b)));

	let img_a = graph.add_node(ImageSolidColorOp::new(
		ValueParam::with_mapping(Value::Vec4([0.0, 0.0, 0.0, 0.0]), color_a_node),
		config.width,
		config.height,
	));
	let img_b = graph.add_node(ImageSolidColorOp::new(
		ValueParam::with_mapping(Value::Vec4([0.0, 0.0, 0.0, 0.0]), color_b_node),
		config.width,
		config.height,
	));

	let mix_node = graph.add_node(ImageMixOp::new(
		img_a,
		img_b,
		ValueParam::with_mapping(Value::Float(0.0), lfo_node),
	));

	let blur_node = graph.add_node(ImageBlur1DOp::new(
		mix_node,
		ValueParam::with_mapping(Value::Float(0.0), blur_radius_node),
	));

	let image_delay_node = graph.add_node(ImageDelay1Op::new(blur_node));

	(
		graph,
		GraphIds {
			freq: freq_node,
			phase: phase_node,
			blur_base: blur_base_node,
			blur_scale: blur_scale_node,
			color_a: color_a_node,
			color_b: color_b_node,
			lfo: lfo_node,
			lfo_delay: lfo_delay_node,
			blur_radius: blur_radius_node,
			blur: blur_node,
			image_delay: image_delay_node,
		},
	)
}

fn image_to_frame(image: &Image) -> FrameData {
	let mut pixels = Vec::with_capacity(image.rgba.len());
	for channel in &image.rgba {
		let clamped = channel.clamp(0.0, 1.0);
		let value = (clamped * 255.0).round() as u8;
		pixels.push(value);
	}

	FrameData {
		width: image.width,
		height: image.height,
		pixels: bytes::Bytes::from(pixels),
	}
}
