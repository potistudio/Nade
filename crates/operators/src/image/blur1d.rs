use crate::common::{
	image_input_descriptor, resolve_value_param, set_image_input, set_value_param,
	value_param_descriptor,
};
use crate::hash::image::hash_image;
use crate::hash::value::hash_f32;
use crate::image::blur::blur_1d;
use core::{
	EvalAccess, EvalContext, EvalError, Image, NodeId, NodeState, Operator, Output, OutputType,
	ParamDescriptor, ParamValue, ResolvedOp, ValueKind, ValueParam, ValueParamUi,
};

#[derive(Debug, Clone)]
pub struct ImageBlur1DOp {
	pub input: NodeId,
	pub radius: ValueParam,
}

#[derive(Debug, Clone)]
struct ImageBlur1DResolved {
	input: Image,
	radius: f32,
}

impl ImageBlur1DOp {
	pub fn new(input: NodeId, radius: ValueParam) -> Self {
		Self { input, radius }
	}
}

impl Operator for ImageBlur1DOp {
	fn name(&self) -> &'static str {
		"Image.Blur1D"
	}

	fn output_type(&self) -> OutputType {
		OutputType::Image
	}

	fn parameters(&self) -> Vec<ParamDescriptor> {
		vec![
			image_input_descriptor("Input", self.input),
			value_param_descriptor(
				"Radius",
				&self.radius,
				Some(ValueKind::Float),
				true,
				ValueParamUi::Float,
			),
		]
	}

	fn set_parameter(&mut self, index: usize, value: ParamValue) -> bool {
		match index {
			0 => set_image_input(&mut self.input, value),
			1 => set_value_param(&mut self.radius, value),
			_ => false,
		}
	}

	fn resolve(
		&self,
		eval: &mut dyn EvalAccess,
		time: f64,
		ctx: &EvalContext,
	) -> Result<ResolvedOp, EvalError> {
		let input = eval.eval_image(self.input, time, ctx)?;
		let radius = resolve_value_param(&self.radius, eval, time, ctx)?.as_f32()?;
		let input_hash = hash_image(&input);
		let param_hash = hash_f32(radius);
		Ok(ResolvedOp::new(
			ImageBlur1DResolved { input, radius },
			param_hash,
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
		let resolved = resolved.take::<ImageBlur1DResolved>()?;
		let image = blur_1d(&resolved.input, resolved.radius);
		Ok((Output::Image(image), NodeState::Empty))
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
