use crate::common::{resolve_value_param, set_value_param, value_param_descriptor};
use crate::hash::value::hash_select_inputs;
use core::{
	EvalAccess, EvalContext, EvalError, NodeState, Operator, Output, OutputType, ParamDescriptor,
	ParamValue, ResolvedOp, Value, ValueKind, ValueParam, ValueParamUi,
};

#[derive(Debug, Clone)]
pub struct ValueSelectOp {
	pub cond: ValueParam,
	pub a: ValueParam,
	pub b: ValueParam,
}

#[derive(Debug, Clone)]
struct ValueSelectResolved {
	cond: bool,
	a: Value,
	b: Value,
}

impl ValueSelectOp {
	pub fn new(cond: ValueParam, a: ValueParam, b: ValueParam) -> Self {
		Self { cond, a, b }
	}
}

impl Operator for ValueSelectOp {
	fn name(&self) -> &'static str {
		"Value.Select"
	}

	fn output_type(&self) -> OutputType {
		OutputType::Value
	}

	fn value_kind(&self) -> Option<ValueKind> {
		Some(self.a.base.kind())
	}

	fn parameters(&self) -> Vec<ParamDescriptor> {
		vec![
			value_param_descriptor(
				"Condition",
				&self.cond,
				Some(ValueKind::Bool),
				false,
				ValueParamUi::Bool,
			),
			value_param_descriptor("A", &self.a, None, true, ValueParamUi::Default),
			value_param_descriptor("B", &self.b, None, true, ValueParamUi::Default),
		]
	}

	fn set_parameter(&mut self, index: usize, value: ParamValue) -> bool {
		match index {
			0 => set_value_param(&mut self.cond, value),
			1 => set_value_param(&mut self.a, value),
			2 => set_value_param(&mut self.b, value),
			_ => false,
		}
	}

	fn resolve(
		&self,
		eval: &mut dyn EvalAccess,
		time: f64,
		ctx: &EvalContext,
	) -> Result<ResolvedOp, EvalError> {
		let cond = resolve_value_param(&self.cond, eval, time, ctx)?.as_bool()?;
		let a = resolve_value_param(&self.a, eval, time, ctx)?;
		let b = resolve_value_param(&self.b, eval, time, ctx)?;
		if a.kind() != b.kind() {
			return Err(EvalError::ValueSelectMismatch {
				left: a.kind(),
				right: b.kind(),
			});
		}
		let input_hash = hash_select_inputs(cond, a, b);
		Ok(ResolvedOp::new(
			ValueSelectResolved { cond, a, b },
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
		let resolved = resolved.take::<ValueSelectResolved>()?;
		Ok((
			Output::Value(if resolved.cond {
				resolved.a
			} else {
				resolved.b
			}),
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
