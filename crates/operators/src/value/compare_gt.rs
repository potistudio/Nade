use crate::common::{resolve_value_param, set_value_param, value_param_descriptor};
use crate::hash::value::hash_f32_pair;
use core::{
	EvalAccess, EvalContext, EvalError, NodeState, Operator, Output, OutputType, ParamDescriptor, ParamValue,
	ResolvedOp, Value, ValueKind, ValueParam, ValueParamUi,
};

#[derive(Debug, Clone)]
pub struct ValueCompareGTOp {
	pub a: ValueParam,
	pub b: ValueParam,
}

#[derive(Debug, Clone)]
struct ValueCompareGTResolved {
	a: f32,
	b: f32,
}

impl ValueCompareGTOp {
	pub fn new(a: ValueParam, b: ValueParam) -> Self {
		Self { a, b }
	}
}

impl Operator for ValueCompareGTOp {
	fn name(&self) -> &'static str {
		"Value.CompareGT"
	}

	fn output_type(&self) -> OutputType {
		OutputType::Value
	}

	fn value_kind(&self) -> Option<ValueKind> {
		Some(ValueKind::Bool)
	}

	fn parameters(&self) -> Vec<ParamDescriptor> {
		vec![
			value_param_descriptor("A", &self.a, Some(ValueKind::Float), true, ValueParamUi::Float),
			value_param_descriptor("B", &self.b, Some(ValueKind::Float), true, ValueParamUi::Float),
		]
	}

	fn set_parameter(&mut self, index: usize, value: ParamValue) -> bool {
		match index {
			0 => set_value_param(&mut self.a, value),
			1 => set_value_param(&mut self.b, value),
			_ => false,
		}
	}

	fn resolve(&self, eval: &mut dyn EvalAccess, time: f64, ctx: &EvalContext) -> Result<ResolvedOp, EvalError> {
		let a = resolve_value_param(&self.a, eval, time, ctx)?.as_f32()?;
		let b = resolve_value_param(&self.b, eval, time, ctx)?.as_f32()?;
		let input_hash = hash_f32_pair(a, b);
		Ok(ResolvedOp::new(ValueCompareGTResolved { a, b }, 0, input_hash))
	}

	fn compute(
		&self,
		resolved: ResolvedOp,
		_time: f64,
		_state_in: &NodeState,
		_ctx: &EvalContext,
	) -> Result<(Output, NodeState), EvalError> {
		let resolved = resolved.take::<ValueCompareGTResolved>()?;
		Ok((Output::Value(Value::Bool(resolved.a > resolved.b)), NodeState::Empty))
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
