use crate::common::{image_input_descriptor, set_image_input};
use crate::hash::image::hash_image;
use core::{
	EvalAccess, EvalContext, EvalError, Image, NodeId, NodeState, Operator, Output, OutputType, ParamDescriptor,
	ParamValue, ResolvedOp,
};

#[derive(Debug, Clone)]
pub struct ImageDelay1Op {
	pub input: NodeId,
}

impl ImageDelay1Op {
	pub fn new(input: NodeId) -> Self {
		Self { input }
	}
}

impl Operator for ImageDelay1Op {
	fn name(&self) -> &'static str {
		"Image.Delay1"
	}

	fn output_type(&self) -> OutputType {
		OutputType::Image
	}

	fn parameters(&self) -> Vec<ParamDescriptor> {
		vec![image_input_descriptor("Input", self.input)]
	}

	fn set_parameter(&mut self, index: usize, value: ParamValue) -> bool {
		match index {
			0 => set_image_input(&mut self.input, value),
			_ => false,
		}
	}

	fn resolve(&self, eval: &mut dyn EvalAccess, time: f64, ctx: &EvalContext) -> Result<ResolvedOp, EvalError> {
		let input = eval.eval_image(self.input, time, ctx)?;
		let input_hash = hash_image(&input);
		Ok(ResolvedOp::new(input, 0, input_hash))
	}

	fn compute(
		&self,
		resolved: ResolvedOp,
		time: f64,
		state_in: &NodeState,
		_ctx: &EvalContext,
	) -> Result<(Output, NodeState), EvalError> {
		let input = resolved.take::<Image>()?;
		let previous = match state_in {
			NodeState::DelayImage { previous, .. } => previous.clone(),
			_ => None,
		};
		let output = previous.unwrap_or_else(|| input.clone());
		let state_out = NodeState::DelayImage {
			previous: Some(input),
			previous_time: Some(time),
		};
		Ok((Output::Image(output), state_out))
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
