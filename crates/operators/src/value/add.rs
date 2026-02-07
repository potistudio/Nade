use crate::common::{resolve_value_param, set_value_param, value_param_descriptor};
use crate::hash::value::hash_value_pair;
use nade_core::{
	EvalAccess, EvalContext, EvalError, NodeState, Operator, Output, OutputType, ParamDescriptor,
	ParamValue, ResolvedOp, Value, ValueKind, ValueParam, ValueParamUi,
};

#[derive(Debug, Clone)]
pub struct ValueAddOp {
	pub a: ValueParam,
	pub b: ValueParam,
}

#[derive(Debug, Clone)]
struct ValueAddResolved {
	a: Value,
	b: Value,
}

impl ValueAddOp {
	pub fn new(a: ValueParam, b: ValueParam) -> Self {
		Self { a, b }
	}
}

impl Operator for ValueAddOp {
	fn name(&self) -> &'static str {
		"Value.Add"
	}

	fn output_type(&self) -> OutputType {
		OutputType::Value
	}

	fn value_kind(&self) -> Option<ValueKind> {
		Some(self.a.base.kind())
	}

	fn parameters(&self) -> Vec<ParamDescriptor> {
		vec![
			value_param_descriptor("A", &self.a, None, true, ValueParamUi::Default),
			value_param_descriptor("B", &self.b, None, true, ValueParamUi::Default),
		]
	}

	fn set_parameter(&mut self, index: usize, value: ParamValue) -> bool {
		match index {
			0 => set_value_param(&mut self.a, value),
			1 => set_value_param(&mut self.b, value),
			_ => false,
		}
	}

	fn resolve(
		&self,
		eval: &mut dyn EvalAccess,
		time: f64,
		ctx: &EvalContext,
	) -> Result<ResolvedOp, EvalError> {
		let a_val = resolve_value_param(&self.a, eval, time, ctx)?;
		let b_val = resolve_value_param(&self.b, eval, time, ctx)?;
		let input_hash = hash_value_pair(a_val, b_val);
		Ok(ResolvedOp::new(
			ValueAddResolved { a: a_val, b: b_val },
			0,
			input_hash,
		))
	}

	fn compute(
		&self,
		resolved: ResolvedOp,
		_time: f64,
		_state_in: &NodeState,
		_ctx: &EvalContext,
	) -> Result<(Output, NodeState), EvalError> {
		let resolved = resolved.take::<ValueAddResolved>()?;
		Ok((
			Output::Value(resolved.a.add_value(resolved.b)?),
			NodeState::Empty,
		))
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
