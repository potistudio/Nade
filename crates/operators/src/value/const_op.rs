use crate::common::value_param_descriptor;
use crate::hash::value::hash_value;
use core::{
	EvalAccess, EvalContext, EvalError, NodeState, Operator, Output, OutputType, ParamDescriptor, ParamValue,
	ResolvedOp, Value, ValueKind, ValueParam, ValueParamUi,
};

#[derive(Debug, Clone)]
pub struct ValueConstOp {
	pub value: Value,
}

impl ValueConstOp {
	pub fn new(value: Value) -> Self {
		Self { value }
	}
}

impl Operator for ValueConstOp {
	fn name(&self) -> &'static str {
		"Value.Const"
	}

	fn output_type(&self) -> OutputType {
		OutputType::Value
	}

	fn value_kind(&self) -> Option<ValueKind> {
		Some(self.value.kind())
	}

	fn parameters(&self) -> Vec<ParamDescriptor> {
		vec![value_param_descriptor(
			"Value",
			&ValueParam {
				base: self.value,
				mapping: None,
			},
			None,
			false,
			ValueParamUi::Default,
		)]
	}

	fn set_parameter(&mut self, index: usize, value: ParamValue) -> bool {
		match index {
			0 => match value {
				ParamValue::ValueParam(param) => {
					self.value = param.base;
					true
				}
				_ => false,
			},
			_ => false,
		}
	}

	fn resolve(&self, _eval: &mut dyn EvalAccess, _time: f64, _ctx: &EvalContext) -> Result<ResolvedOp, EvalError> {
		Ok(ResolvedOp::new(self.value, hash_value(self.value), 0))
	}

	fn compute(
		&self,
		resolved: ResolvedOp,
		_time: f64,
		_state_in: &NodeState,
		_ctx: &EvalContext,
	) -> Result<(Output, NodeState), EvalError> {
		let value = resolved.take::<Value>()?;
		Ok((Output::Value(value), NodeState::Empty))
	}

	fn clone_box(&self) -> Box<dyn Operator> {
		Box::new(self.clone())
	}

	fn as_any(&self) -> &dyn std::any::Any {
		self
	}

	fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
		self
	}
}
