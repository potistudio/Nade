mod common;
mod hash;
pub mod image;
pub mod value;

use nade_core::{NodeId, Value, ValueParam};

pub use image::{ImageBlur1DOp, ImageDelay1Op, ImageMixOp, ImageSolidColorOp};
pub use value::{
	ValueAddOp, ValueCompareGTOp, ValueConstOp, ValueDelay1Op, ValueLfoOp, ValueMulOp,
	ValueSelectOp, ValueSinOp,
};

pub use nade_core::{EvalAccess, Operator, ResolvedHashes, ResolvedOp};

pub struct OperatorDefinition {
	pub key: &'static str,
	pub label: &'static str,
	creator: fn() -> Box<dyn Operator>,
}

impl OperatorDefinition {
	pub fn create(&self) -> Box<dyn Operator> {
		(self.creator)()
	}
}

fn create_value_const() -> Box<dyn Operator> {
	Box::new(ValueConstOp::new(Value::Float(0.0)))
}

fn create_value_add() -> Box<dyn Operator> {
	Box::new(ValueAddOp::new(
		ValueParam::new(Value::Float(0.0)),
		ValueParam::new(Value::Float(0.0)),
	))
}

fn create_value_mul() -> Box<dyn Operator> {
	Box::new(ValueMulOp::new(
		ValueParam::new(Value::Float(1.0)),
		ValueParam::new(Value::Float(1.0)),
	))
}

fn create_value_sin() -> Box<dyn Operator> {
	Box::new(ValueSinOp::new(
		ValueParam::new(Value::Float(1.0)),
		ValueParam::new(Value::Float(0.0)),
	))
}

fn create_value_compare_gt() -> Box<dyn Operator> {
	Box::new(ValueCompareGTOp::new(
		ValueParam::new(Value::Float(0.0)),
		ValueParam::new(Value::Float(0.0)),
	))
}

fn create_value_select() -> Box<dyn Operator> {
	Box::new(ValueSelectOp::new(
		ValueParam::new(Value::Bool(false)),
		ValueParam::new(Value::Float(0.0)),
		ValueParam::new(Value::Float(1.0)),
	))
}

fn create_value_delay_1() -> Box<dyn Operator> {
	Box::new(ValueDelay1Op::new(ValueParam::new(Value::Float(0.0))))
}

fn create_value_lfo() -> Box<dyn Operator> {
	Box::new(ValueLfoOp::new(
		ValueParam::new(Value::Float(1.0)),
		ValueParam::new(Value::Float(1.0)),
		ValueParam::new(Value::Float(0.0)),
	))
}

fn create_image_solid_color() -> Box<dyn Operator> {
	Box::new(ImageSolidColorOp::new(
		ValueParam::new(Value::Vec4([0.2, 0.2, 0.2, 1.0])),
		256,
		256,
	))
}

fn create_image_mix() -> Box<dyn Operator> {
	Box::new(ImageMixOp::new(
		NodeId::new(1),
		NodeId::new(1),
		ValueParam::new(Value::Float(0.5)),
	))
}

fn create_image_blur_1d() -> Box<dyn Operator> {
	Box::new(ImageBlur1DOp::new(
		NodeId::new(1),
		ValueParam::new(Value::Float(4.0)),
	))
}

fn create_image_delay_1() -> Box<dyn Operator> {
	Box::new(ImageDelay1Op::new(NodeId::new(1)))
}

static OPERATOR_DEFINITIONS: [OperatorDefinition; 12] = [
	OperatorDefinition {
		key: "Value.Const",
		label: "Value.Const",
		creator: create_value_const,
	},
	OperatorDefinition {
		key: "Value.Add",
		label: "Value.Add",
		creator: create_value_add,
	},
	OperatorDefinition {
		key: "Value.Mul",
		label: "Value.Mul",
		creator: create_value_mul,
	},
	OperatorDefinition {
		key: "Value.Sin",
		label: "Value.Sin",
		creator: create_value_sin,
	},
	OperatorDefinition {
		key: "Value.CompareGT",
		label: "Value.CompareGT",
		creator: create_value_compare_gt,
	},
	OperatorDefinition {
		key: "Value.Select",
		label: "Value.Select",
		creator: create_value_select,
	},
	OperatorDefinition {
		key: "Value.Delay1",
		label: "Value.Delay1",
		creator: create_value_delay_1,
	},
	OperatorDefinition {
		key: "Value.LFO",
		label: "Value.LFO",
		creator: create_value_lfo,
	},
	OperatorDefinition {
		key: "Image.SolidColor",
		label: "Image.SolidColor",
		creator: create_image_solid_color,
	},
	OperatorDefinition {
		key: "Image.Mix",
		label: "Image.Mix",
		creator: create_image_mix,
	},
	OperatorDefinition {
		key: "Image.Blur1D",
		label: "Image.Blur1D",
		creator: create_image_blur_1d,
	},
	OperatorDefinition {
		key: "Image.Delay1",
		label: "Image.Delay1",
		creator: create_image_delay_1,
	},
];

pub fn operator_definitions() -> &'static [OperatorDefinition] {
	&OPERATOR_DEFINITIONS
}

pub fn create_operator(key: &str) -> Option<Box<dyn Operator>> {
	operator_definitions()
		.iter()
		.find(|definition| definition.key == key)
		.map(OperatorDefinition::create)
}

pub fn operator_label(key: &str) -> &str {
	operator_definitions()
		.iter()
		.find(|definition| definition.key == key)
		.map(|definition| definition.label)
		.unwrap_or(key)
}

pub fn next_operator_key(current: &str) -> Option<&'static str> {
	let definitions = operator_definitions();
	let current_index = definitions
		.iter()
		.position(|definition| definition.key == current)?;
	Some(definitions[(current_index + 1) % definitions.len()].key)
}

pub fn prev_operator_key(current: &str) -> Option<&'static str> {
	let definitions = operator_definitions();
	let current_index = definitions
		.iter()
		.position(|definition| definition.key == current)?;
	Some(definitions[(current_index + definitions.len() - 1) % definitions.len()].key)
}
