use crate::common::{resolve_value_param, set_value_param, value_param_descriptor};
use crate::hash::value::{hash_f32, hash_f32_pair};
use core::{
	EvalAccess, EvalContext, EvalError, NodeState, Operator, Output, OutputType, ParamDescriptor, ParamValue,
	ResolvedOp, Value, ValueKind, ValueParam, ValueParamUi,
};

#[derive(Debug, Clone)]
pub struct ValueLfoOp {
	pub frequency: ValueParam,
	pub amplitude: ValueParam,
	pub phase: ValueParam,
}

#[derive(Debug, Clone)]
struct ValueLfoResolved {
	frequency: f32,
	amplitude: f32,
	phase: f32,
}

impl ValueLfoOp {
	pub fn new(frequency: ValueParam, amplitude: ValueParam, phase: ValueParam) -> Self {
		Self {
			frequency,
			amplitude,
			phase,
		}
	}
}

impl Operator for ValueLfoOp {
	fn name(&self) -> &'static str {
		"Value.LFO"
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
				&self.frequency,
				Some(ValueKind::Float),
				true,
				ValueParamUi::Float,
			),
			value_param_descriptor(
				"Amplitude",
				&self.amplitude,
				Some(ValueKind::Float),
				true,
				ValueParamUi::Float,
			),
			value_param_descriptor("Phase", &self.phase, Some(ValueKind::Float), true, ValueParamUi::Float),
		]
	}

	fn set_parameter(&mut self, index: usize, value: ParamValue) -> bool {
		match index {
			0 => set_value_param(&mut self.frequency, value),
			1 => set_value_param(&mut self.amplitude, value),
			2 => set_value_param(&mut self.phase, value),
			_ => false,
		}
	}

	fn resolve(&self, eval: &mut dyn EvalAccess, time: f64, ctx: &EvalContext) -> Result<ResolvedOp, EvalError> {
		let frequency = resolve_value_param(&self.frequency, eval, time, ctx)?.as_f32()?;
		let amplitude = resolve_value_param(&self.amplitude, eval, time, ctx)?.as_f32()?;
		let phase = resolve_value_param(&self.phase, eval, time, ctx)?.as_f32()?;
		let param_hash = hash_f32_pair(frequency, amplitude) ^ hash_f32(phase);
		Ok(ResolvedOp::new(
			ValueLfoResolved {
				frequency,
				amplitude,
				phase,
			},
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
		let resolved = resolved.take::<ValueLfoResolved>()?;
		let angle = std::f64::consts::TAU * f64::from(resolved.frequency) * time + f64::from(resolved.phase);
		#[allow(clippy::cast_precision_loss)]
		let output = Value::Float((angle.sin() as f32) * resolved.amplitude);
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
