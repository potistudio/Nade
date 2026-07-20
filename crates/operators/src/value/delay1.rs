use crate::common::{resolve_value_param, set_value_param, value_param_descriptor};
use crate::hash::value::hash_value;
use core::{
	EvalAccess, EvalContext, EvalError, NodeState, Operator, Output, OutputType, ParamDescriptor, ParamValue,
	ResolvedOp, Value, ValueKind, ValueParam, ValueParamUi,
};

#[derive(Debug, Clone)]
pub struct ValueDelay1Op {
	pub input: ValueParam,
}

impl ValueDelay1Op {
	pub fn new(input: ValueParam) -> Self {
		Self { input }
	}
}

impl Operator for ValueDelay1Op {
	fn name(&self) -> &'static str {
		"Value.Delay1"
	}

	fn output_type(&self) -> OutputType {
		OutputType::Value
	}

	fn value_kind(&self) -> Option<ValueKind> {
		Some(self.input.base.kind())
	}

	fn parameters(&self) -> Vec<ParamDescriptor> {
		vec![value_param_descriptor(
			"Input",
			&self.input,
			None,
			true,
			ValueParamUi::Default,
		)]
	}

	fn set_parameter(&mut self, index: usize, value: ParamValue) -> bool {
		match index {
			0 => set_value_param(&mut self.input, value),
			_ => false,
		}
	}

	fn resolve(&self, eval: &mut dyn EvalAccess, time: f64, ctx: &EvalContext) -> Result<ResolvedOp, EvalError> {
		let input = resolve_value_param(&self.input, eval, time, ctx)?;
		let input_hash = hash_value(input);
		Ok(ResolvedOp::new(input, 0, input_hash))
	}

	fn compute(
		&self,
		resolved: ResolvedOp,
		time: f64,
		state_in: &NodeState,
		_ctx: &EvalContext,
	) -> Result<(Output, NodeState), EvalError> {
		let input = resolved.take::<Value>()?;
		let previous = match state_in {
			NodeState::DelayValue { previous, .. } => *previous,
			_ => None,
		};
		let output = previous.unwrap_or(input);
		let state_out = NodeState::DelayValue {
			previous: Some(input),
			previous_time: Some(time),
		};
		Ok((Output::Value(output), state_out))
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
