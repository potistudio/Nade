use crate::common::{resolve_value_param, set_value_param, value_param_descriptor};
use crate::hash::value::hash_f32_pair;
use core::{
	EvalAccess, EvalContext, EvalError, NodeState, Operator, Output, OutputType, ParamDescriptor,
	ParamValue, ResolvedOp, Value, ValueKind, ValueParam, ValueParamUi,
};

#[derive(Debug, Clone)]
pub struct ValueSinOp {
	pub freq: ValueParam,
	pub phase: ValueParam,
}

#[derive(Debug, Clone)]
struct ValueSinResolved {
	freq: f32,
	phase: f32,
}

impl ValueSinOp {
	pub fn new(freq: ValueParam, phase: ValueParam) -> Self {
		Self { freq, phase }
	}
}

impl Operator for ValueSinOp {
	fn name(&self) -> &'static str {
		"Value.Sin"
	}

	fn output_type(&self) -> OutputType {
		OutputType::Value
	}

	fn value_kind(&self) -> Option<ValueKind> {
		Some(ValueKind::Float)
	}

	fn parameters(&self) -> Vec<ParamDescriptor> {
		vec![
			value_param_descriptor(
				"Frequency",
				&self.freq,
				Some(ValueKind::Float),
				true,
				ValueParamUi::Float,
			),
			value_param_descriptor(
				"Phase",
				&self.phase,
				Some(ValueKind::Float),
				true,
				ValueParamUi::Float,
			),
		]
	}

	fn set_parameter(&mut self, index: usize, value: ParamValue) -> bool {
		match index {
			0 => set_value_param(&mut self.freq, value),
			1 => set_value_param(&mut self.phase, value),
			_ => false,
		}
	}

	fn resolve(
		&self,
		eval: &mut dyn EvalAccess,
		time: f64,
		ctx: &EvalContext,
	) -> Result<ResolvedOp, EvalError> {
		let freq = resolve_value_param(&self.freq, eval, time, ctx)?.as_f32()?;
		let phase = resolve_value_param(&self.phase, eval, time, ctx)?.as_f32()?;
		let param_hash = hash_f32_pair(freq, phase);
		Ok(ResolvedOp::new(
			ValueSinResolved { freq, phase },
			param_hash,
			0,
		))
	}

	fn compute(
		&self,
		resolved: ResolvedOp,
		time: f64,
		_state_in: &NodeState,
		_ctx: &EvalContext,
	) -> Result<(Output, NodeState), EvalError> {
		let resolved = resolved.take::<ValueSinResolved>()?;
		let angle =
			std::f64::consts::TAU * f64::from(resolved.freq) * time + f64::from(resolved.phase);
		#[allow(clippy::cast_precision_loss)]
		let output = Value::Float(angle.sin() as f32);
		Ok((Output::Value(output), NodeState::Empty))
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
