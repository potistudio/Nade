mod draggable_number;

use draggable_number::draggable_number;
use iced::widget::button as button_widget;
use iced::widget::{button, checkbox, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};
use nade_core::{
	Graph, NodeId, NodeState, Operator, OutputType, ParamDescriptor, ParamKind, ParamValue, Value,
	ValueParam, ValueParamUi,
};
use operators::{
	ImageBlur1DOp, ImageDelay1Op, ImageMixOp, ImageSolidColorOp, ValueAddOp, ValueCompareGTOp,
	ValueConstOp, ValueDelay1Op, ValueLfoOp, ValueMulOp, ValueSelectOp, ValueSinOp,
};

const INSPECTOR_BG: iced::Color = iced::Color::from_rgb(0.128, 0.128, 0.128);
const TEXT_PRIMARY: iced::Color = iced::Color::WHITE;
const TEXT_SECONDARY: iced::Color = iced::Color::from_rgb(0.8, 0.8, 0.8);
const TEXT_MUTED: iced::Color = iced::Color::from_rgb(0.6, 0.6, 0.6);

#[derive(Debug, Clone)]
pub enum InspectorMessage {
	AddOperator,
	CycleOperatorPrev(NodeId),
	CycleOperatorNext(NodeId),
	SetOperatorParameter {
		node_id: NodeId,
		index: usize,
		value: ParamValue,
	},
}

#[derive(Debug)]
pub struct InspectorUiState {
	graph: Graph,
	operator_ids: Vec<NodeId>,
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
	ValueLfo,
	ImageSolidColor,
	ImageMix,
	ImageBlur1D,
	ImageDelay1,
}

impl OperatorTemplate {
	const ALL: [Self; 12] = [
		Self::ValueConst,
		Self::ValueAdd,
		Self::ValueMul,
		Self::ValueSin,
		Self::ValueCompareGT,
		Self::ValueSelect,
		Self::ValueDelay1,
		Self::ValueLfo,
		Self::ImageSolidColor,
		Self::ImageMix,
		Self::ImageBlur1D,
		Self::ImageDelay1,
	];

	fn all() -> &'static [Self] {
		&Self::ALL
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
			Self::ValueLfo => "Value.LFO",
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
		} else if operator.as_any().is::<ValueLfoOp>() {
			Some(Self::ValueLfo)
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

	fn next(self) -> Self {
		let all = Self::all();
		let current = all
			.iter()
			.position(|template| *template == self)
			.unwrap_or(0);
		all[(current + 1) % all.len()]
	}

	fn prev(self) -> Self {
		let all = Self::all();
		let current = all
			.iter()
			.position(|template| *template == self)
			.unwrap_or(0);
		all[(current + all.len() - 1) % all.len()]
	}
}

impl InspectorUiState {
	pub fn new() -> Self {
		Self {
			graph: Graph::new(),
			operator_ids: Vec::new(),
		}
	}

	pub fn update(&mut self, msg: InspectorMessage) {
		match msg {
			InspectorMessage::AddOperator => {
				self.add_operator(OperatorTemplate::ValueConst);
			}
			InspectorMessage::CycleOperatorPrev(node_id) => {
				self.cycle_operator(node_id, false);
			}
			InspectorMessage::CycleOperatorNext(node_id) => {
				self.cycle_operator(node_id, true);
			}
			InspectorMessage::SetOperatorParameter {
				node_id,
				index,
				value,
			} => {
				self.set_parameter(node_id, index, value);
			}
		}
	}

	pub fn evaluate(&mut self, _time: f64) {}

	fn operator_ids(&self) -> &[NodeId] {
		&self.operator_ids
	}

	fn graph(&self) -> &Graph {
		&self.graph
	}

	fn add_operator(&mut self, template: OperatorTemplate) {
		let operator = self.make_operator(template, None);
		let node_id = self.graph.add_node_boxed(operator);
		self.operator_ids.push(node_id);
		self.graph.clear_cache();
	}

	fn cycle_operator(&mut self, node_id: NodeId, forward: bool) {
		let current = self
			.graph
			.node(node_id)
			.and_then(|node| OperatorTemplate::from_operator(node.operator.as_ref()))
			.unwrap_or(OperatorTemplate::ValueConst);
		let next = if forward {
			current.next()
		} else {
			current.prev()
		};
		let replacement = self.make_operator(next, Some(node_id));

		if let Some(node) = self.graph.node_mut(node_id) {
			node.operator = replacement;
			node.state = NodeState::default();
			self.graph.clear_cache();
		}
	}

	fn set_parameter(&mut self, node_id: NodeId, index: usize, value: ParamValue) {
		if let Some(node) = self.graph.node_mut(node_id)
			&& node.operator.set_parameter(index, value)
		{
			node.state = NodeState::default();
			self.graph.clear_cache();
		}
	}

	fn make_operator(
		&self,
		template: OperatorTemplate,
		node_hint: Option<NodeId>,
	) -> Box<dyn Operator> {
		let fallback = node_hint
			.or_else(|| self.operator_ids.last().copied())
			.unwrap_or_else(|| NodeId::new(1));

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
			OperatorTemplate::ValueLfo => Box::new(ValueLfoOp::new(
				ValueParam::new(Value::Float(1.0)),
				ValueParam::new(Value::Float(1.0)),
				ValueParam::new(Value::Float(0.0)),
			)),
			OperatorTemplate::ImageSolidColor => Box::new(ImageSolidColorOp::new(
				ValueParam::new(Value::Vec4([0.2, 0.2, 0.2, 1.0])),
				256,
				256,
			)),
			OperatorTemplate::ImageMix => {
				let (a, b) = self.default_image_pair(fallback);
				Box::new(ImageMixOp::new(a, b, ValueParam::new(Value::Float(0.5))))
			}
			OperatorTemplate::ImageBlur1D => {
				let input = self.default_image_input(fallback);
				Box::new(ImageBlur1DOp::new(
					input,
					ValueParam::new(Value::Float(4.0)),
				))
			}
			OperatorTemplate::ImageDelay1 => {
				let input = self.default_image_input(fallback);
				Box::new(ImageDelay1Op::new(input))
			}
		}
	}

	fn default_image_input(&self, fallback: NodeId) -> NodeId {
		self.operator_ids
			.iter()
			.copied()
			.find(|node_id| {
				self.graph
					.node(*node_id)
					.is_some_and(|node| node.operator.output_type() == OutputType::Image)
			})
			.unwrap_or(fallback)
	}

	fn default_image_pair(&self, fallback: NodeId) -> (NodeId, NodeId) {
		let mut images = self.operator_ids.iter().copied().filter(|node_id| {
			self.graph
				.node(*node_id)
				.is_some_and(|node| node.operator.output_type() == OutputType::Image)
		});

		let first = images.next().unwrap_or(fallback);
		let second = images.next().unwrap_or(first);
		(first, second)
	}
}

impl Default for InspectorUiState {
	fn default() -> Self {
		Self::new()
	}
}

pub fn view<'a>(state: &'a InspectorUiState) -> Element<'a, InspectorMessage> {
	let add_button = add_operator_button();

	let body: Element<'_, InspectorMessage> = if state.operator_ids().is_empty() {
		container(add_button)
			.width(Length::Fill)
			.height(Length::Fill)
			.center_x(Length::Fill)
			.center_y(Length::Fill)
			.into()
	} else {
		let mut list = column![add_button].spacing(10).width(Length::Fill);
		for node_id in state.operator_ids() {
			if let Some(node) = state.graph().node(*node_id) {
				list = list.push(operator_card(*node_id, node.operator.as_ref()));
			}
		}

		scrollable(container(list).width(Length::Fill).padding([0, 2]))
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	};

	container(body)
		.width(Length::Fill)
		.height(Length::Fill)
		.padding(12)
		.style(|_| container::Style {
			background: Some(INSPECTOR_BG.into()),
			..Default::default()
		})
		.into()
}

fn add_operator_button<'a>() -> Element<'a, InspectorMessage> {
	button(text("Add Operator").size(12).color(TEXT_PRIMARY))
		.on_press(InspectorMessage::AddOperator)
		.padding([8, 14])
		.style(outlined_button_style)
		.into()
}

fn operator_card<'a>(node_id: NodeId, operator: &dyn Operator) -> Element<'a, InspectorMessage> {
	let operator_name = OperatorTemplate::from_operator(operator)
		.map(OperatorTemplate::label)
		.unwrap_or(operator.name());

	let mut parameters = column![].spacing(8).width(Length::Fill);
	let mut has_parameter = false;
	for (index, descriptor) in operator.parameters().into_iter().enumerate() {
		has_parameter = true;
		parameters = parameters.push(parameter_editor(node_id, index, descriptor));
	}
	if !has_parameter {
		parameters = parameters.push(text("No parameters").size(12).color(TEXT_MUTED));
	}

	let header = row![
		text(format!("#{}", node_id.value()))
			.size(11)
			.color(TEXT_SECONDARY),
		text(operator_name).size(13).color(TEXT_PRIMARY),
		row![
			button(text("<").size(12))
				.on_press(InspectorMessage::CycleOperatorPrev(node_id))
				.padding([2, 8])
				.style(flat_button_style),
			button(text(">").size(12))
				.on_press(InspectorMessage::CycleOperatorNext(node_id))
				.padding([2, 8])
				.style(flat_button_style),
		]
		.spacing(4),
	]
	.spacing(8)
	.align_y(Alignment::Center);

	container(column![header, parameters].spacing(8).width(Length::Fill))
		.width(Length::Fill)
		.padding(10)
		.style(|_| container::Style {
			background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.08).into()),
			border: iced::Border {
				color: iced::Color::from_rgb(0.35, 0.35, 0.35),
				width: 1.0,
				radius: 6.0.into(),
			},
			..Default::default()
		})
		.into()
}

fn parameter_editor<'a>(
	node_id: NodeId,
	index: usize,
	descriptor: ParamDescriptor,
) -> Element<'a, InspectorMessage> {
	match (descriptor.kind, descriptor.value) {
		(ParamKind::ValueParam { ui, .. }, ParamValue::ValueParam(param)) => {
			value_param_editor(node_id, index, descriptor.label, ui, param)
		}
		(ParamKind::U32 { min, max }, ParamValue::U32(value)) => {
			u32_param_editor(node_id, index, descriptor.label, min, max, value)
		}
		(ParamKind::ImageInput, ParamValue::ImageInput(input)) => {
			image_input_editor(node_id, index, descriptor.label, input)
		}
		_ => text(format!("{}: unsupported parameter", descriptor.label))
			.size(12)
			.color(TEXT_MUTED)
			.into(),
	}
}

fn value_param_editor<'a>(
	node_id: NodeId,
	index: usize,
	label: String,
	ui_hint: ValueParamUi,
	param: ValueParam,
) -> Element<'a, InspectorMessage> {
	let mapping = param.mapping;

	let editor = match (ui_hint, param.base) {
		(ValueParamUi::Bool, Value::Bool(value)) => {
			bool_param_editor(node_id, index, label, param, value)
		}
		(ValueParamUi::Color, Value::Vec4(value)) => {
			color_param_editor(node_id, index, label, param, value)
		}
		(_, Value::Float(value)) => float_param_editor(node_id, index, label, param, value),
		(_, Value::Vec2(value)) => vec2_param_editor(node_id, index, label, param, value),
		(_, Value::Vec3(value)) => vec3_param_editor(node_id, index, label, param, value),
		(_, Value::Vec4(value)) => vec4_param_editor(node_id, index, label, param, value),
		(_, Value::Bool(value)) => bool_param_editor(node_id, index, label, param, value),
	};

	let mut content = column![editor].spacing(4);
	if let Some(mapped_id) = mapping {
		content = content.push(
			text(format!("map -> #{}", mapped_id.value()))
				.size(11)
				.color(TEXT_SECONDARY),
		);
	}

	content.into()
}

fn float_param_editor<'a>(
	node_id: NodeId,
	index: usize,
	label: String,
	param: ValueParam,
	value: f32,
) -> Element<'a, InspectorMessage> {
	row![
		text(label)
			.size(12)
			.color(TEXT_MUTED)
			.width(Length::Fixed(130.0)),
		draggable_number(value, 0.0, move |next| {
			InspectorMessage::SetOperatorParameter {
				node_id,
				index,
				value: set_base_value(param.clone(), Value::Float(next)),
			}
		})
		.step(0.01)
		.width(Length::Fill),
	]
	.spacing(10)
	.align_y(Alignment::Center)
	.into()
}

fn vec2_param_editor<'a>(
	node_id: NodeId,
	index: usize,
	label: String,
	param: ValueParam,
	value: [f32; 2],
) -> Element<'a, InspectorMessage> {
	row![
		text(label)
			.size(12)
			.color(TEXT_MUTED)
			.width(Length::Fixed(130.0)),
		row![
			axis_editor("X", value[0], {
				let param = param.clone();
				move |next| InspectorMessage::SetOperatorParameter {
					node_id,
					index,
					value: set_vec2_axis(param.clone(), 0, next),
				}
			}),
			axis_editor("Y", value[1], move |next| {
				InspectorMessage::SetOperatorParameter {
					node_id,
					index,
					value: set_vec2_axis(param.clone(), 1, next),
				}
			}),
		]
		.spacing(8)
	]
	.spacing(10)
	.align_y(Alignment::Center)
	.into()
}

fn vec3_param_editor<'a>(
	node_id: NodeId,
	index: usize,
	label: String,
	param: ValueParam,
	value: [f32; 3],
) -> Element<'a, InspectorMessage> {
	row![
		text(label)
			.size(12)
			.color(TEXT_MUTED)
			.width(Length::Fixed(130.0)),
		row![
			axis_editor("X", value[0], {
				let param = param.clone();
				move |next| InspectorMessage::SetOperatorParameter {
					node_id,
					index,
					value: set_vec3_axis(param.clone(), 0, next),
				}
			}),
			axis_editor("Y", value[1], {
				let param = param.clone();
				move |next| InspectorMessage::SetOperatorParameter {
					node_id,
					index,
					value: set_vec3_axis(param.clone(), 1, next),
				}
			}),
			axis_editor("Z", value[2], move |next| {
				InspectorMessage::SetOperatorParameter {
					node_id,
					index,
					value: set_vec3_axis(param.clone(), 2, next),
				}
			}),
		]
		.spacing(8)
	]
	.spacing(10)
	.align_y(Alignment::Center)
	.into()
}

fn vec4_param_editor<'a>(
	node_id: NodeId,
	index: usize,
	label: String,
	param: ValueParam,
	value: [f32; 4],
) -> Element<'a, InspectorMessage> {
	column![
		text(label).size(12).color(TEXT_MUTED),
		row![
			axis_editor("X", value[0], {
				let param = param.clone();
				move |next| InspectorMessage::SetOperatorParameter {
					node_id,
					index,
					value: set_vec4_axis(param.clone(), 0, next),
				}
			}),
			axis_editor("Y", value[1], {
				let param = param.clone();
				move |next| InspectorMessage::SetOperatorParameter {
					node_id,
					index,
					value: set_vec4_axis(param.clone(), 1, next),
				}
			}),
			axis_editor("Z", value[2], {
				let param = param.clone();
				move |next| InspectorMessage::SetOperatorParameter {
					node_id,
					index,
					value: set_vec4_axis(param.clone(), 2, next),
				}
			}),
			axis_editor("W", value[3], move |next| {
				InspectorMessage::SetOperatorParameter {
					node_id,
					index,
					value: set_vec4_axis(param.clone(), 3, next),
				}
			}),
		]
		.spacing(8)
	]
	.spacing(6)
	.into()
}

fn color_param_editor<'a>(
	node_id: NodeId,
	index: usize,
	label: String,
	param: ValueParam,
	value: [f32; 4],
) -> Element<'a, InspectorMessage> {
	let swatch = container(text(""))
		.width(Length::Fixed(18.0))
		.height(Length::Fixed(18.0))
		.style(move |_| container::Style {
			background: Some(iced::Color::from_rgba(value[0], value[1], value[2], value[3]).into()),
			border: iced::Border {
				color: iced::Color::from_rgb(0.35, 0.35, 0.35),
				width: 1.0,
				radius: 3.0.into(),
			},
			..Default::default()
		});

	column![
		row![text(label).size(12).color(TEXT_MUTED), swatch]
			.spacing(8)
			.align_y(Alignment::Center),
		row![
			axis_editor("R", value[0], {
				let param = param.clone();
				move |next| InspectorMessage::SetOperatorParameter {
					node_id,
					index,
					value: set_vec4_axis(param.clone(), 0, next.clamp(0.0, 1.0)),
				}
			}),
			axis_editor("G", value[1], {
				let param = param.clone();
				move |next| InspectorMessage::SetOperatorParameter {
					node_id,
					index,
					value: set_vec4_axis(param.clone(), 1, next.clamp(0.0, 1.0)),
				}
			}),
			axis_editor("B", value[2], {
				let param = param.clone();
				move |next| InspectorMessage::SetOperatorParameter {
					node_id,
					index,
					value: set_vec4_axis(param.clone(), 2, next.clamp(0.0, 1.0)),
				}
			}),
			axis_editor("A", value[3], move |next| {
				InspectorMessage::SetOperatorParameter {
					node_id,
					index,
					value: set_vec4_axis(param.clone(), 3, next.clamp(0.0, 1.0)),
				}
			}),
		]
		.spacing(8)
	]
	.spacing(6)
	.into()
}

fn bool_param_editor<'a>(
	node_id: NodeId,
	index: usize,
	label: String,
	param: ValueParam,
	value: bool,
) -> Element<'a, InspectorMessage> {
	checkbox(value)
		.label(label)
		.on_toggle(move |enabled| InspectorMessage::SetOperatorParameter {
			node_id,
			index,
			value: set_base_value(param.clone(), Value::Bool(enabled)),
		})
		.into()
}

fn u32_param_editor<'a>(
	node_id: NodeId,
	index: usize,
	label: String,
	min: u32,
	max: u32,
	value: u32,
) -> Element<'a, InspectorMessage> {
	row![
		text(label)
			.size(12)
			.color(TEXT_MUTED)
			.width(Length::Fixed(130.0)),
		draggable_number(value as f32, value as f32, move |next| {
			let clamped = next.round().clamp(min as f32, max as f32) as u32;
			InspectorMessage::SetOperatorParameter {
				node_id,
				index,
				value: ParamValue::U32(clamped),
			}
		})
		.step(1.0)
		.width(Length::Fill),
	]
	.spacing(10)
	.align_y(Alignment::Center)
	.into()
}

fn image_input_editor<'a>(
	node_id: NodeId,
	index: usize,
	label: String,
	input: NodeId,
) -> Element<'a, InspectorMessage> {
	row![
		text(label)
			.size(12)
			.color(TEXT_MUTED)
			.width(Length::Fixed(130.0)),
		draggable_number(input.value() as f32, input.value() as f32, move |next| {
			let id = next.max(1.0).round() as usize;
			InspectorMessage::SetOperatorParameter {
				node_id,
				index,
				value: ParamValue::ImageInput(NodeId::new(id)),
			}
		})
		.step(1.0)
		.width(Length::Fill),
	]
	.spacing(10)
	.align_y(Alignment::Center)
	.into()
}

fn axis_editor<'a>(
	axis: &'a str,
	value: f32,
	on_change: impl Fn(f32) -> InspectorMessage + 'a,
) -> Element<'a, InspectorMessage> {
	row![
		text(axis).size(11).color(TEXT_SECONDARY),
		draggable_number(value, 0.0, on_change)
			.step(0.01)
			.width(Length::Fixed(78.0)),
	]
	.spacing(5)
	.align_y(Alignment::Center)
	.into()
}

fn set_base_value(mut param: ValueParam, value: Value) -> ParamValue {
	param.base = value;
	ParamValue::ValueParam(param)
}

fn set_vec2_axis(mut param: ValueParam, axis: usize, value: f32) -> ParamValue {
	if let Value::Vec2(mut values) = param.base
		&& axis < 2
	{
		values[axis] = value;
		param.base = Value::Vec2(values);
	}
	ParamValue::ValueParam(param)
}

fn set_vec3_axis(mut param: ValueParam, axis: usize, value: f32) -> ParamValue {
	if let Value::Vec3(mut values) = param.base
		&& axis < 3
	{
		values[axis] = value;
		param.base = Value::Vec3(values);
	}
	ParamValue::ValueParam(param)
}

fn set_vec4_axis(mut param: ValueParam, axis: usize, value: f32) -> ParamValue {
	if let Value::Vec4(mut values) = param.base
		&& axis < 4
	{
		values[axis] = value;
		param.base = Value::Vec4(values);
	}
	ParamValue::ValueParam(param)
}

fn outlined_button_style(
	_theme: &iced::Theme,
	status: button_widget::Status,
) -> button_widget::Style {
	let border_color = match status {
		button_widget::Status::Hovered => iced::Color::from_rgb(0.6, 0.6, 0.6),
		_ => iced::Color::from_rgb(0.4, 0.4, 0.4),
	};

	button_widget::Style {
		background: None,
		text_color: TEXT_PRIMARY,
		border: iced::Border {
			color: border_color,
			width: 1.0,
			radius: 6.0.into(),
		},
		..Default::default()
	}
}

fn flat_button_style(_theme: &iced::Theme, status: button_widget::Status) -> button_widget::Style {
	let border_color = match status {
		button_widget::Status::Hovered => iced::Color::from_rgb(0.5, 0.5, 0.5),
		_ => iced::Color::from_rgb(0.35, 0.35, 0.35),
	};

	button_widget::Style {
		background: None,
		text_color: TEXT_SECONDARY,
		border: iced::Border {
			color: border_color,
			width: 1.0,
			radius: 4.0.into(),
		},
		..Default::default()
	}
}
