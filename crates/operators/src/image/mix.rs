use crate::common::{
	clamp01, image_input_descriptor, resolve_value_param, set_image_input, set_value_param,
	value_param_descriptor,
};
use crate::hash::image::hash_image_pair;
use crate::hash::value::hash_f32;
use core::{
	EvalAccess, EvalContext, EvalError, Image, NodeId, NodeState, Operator, Output, OutputType,
	ParamDescriptor, ParamValue, ResolvedOp, ValueKind, ValueParam, ValueParamUi,
};

#[derive(Debug, Clone)]
pub struct ImageMixOp {
	pub a: NodeId,
	pub b: NodeId,
	pub alpha: ValueParam,
}

#[derive(Debug, Clone)]
struct ImageMixResolved {
	a: Image,
	b: Image,
	alpha: f32,
}

impl ImageMixOp {
	pub fn new(a: NodeId, b: NodeId, alpha: ValueParam) -> Self {
		Self { a, b, alpha }
	}
}

impl Operator for ImageMixOp {
	fn name(&self) -> &'static str {
		"Image.Mix"
	}

	fn output_type(&self) -> OutputType {
		OutputType::Image
	}

	fn parameters(&self) -> Vec<ParamDescriptor> {
		vec![
			image_input_descriptor("Input A", self.a),
			image_input_descriptor("Input B", self.b),
			value_param_descriptor(
				"Alpha",
				&self.alpha,
				Some(ValueKind::Float),
				true,
				ValueParamUi::Float,
			),
		]
	}

	fn set_parameter(&mut self, index: usize, value: ParamValue) -> bool {
		match index {
			0 => set_image_input(&mut self.a, value),
			1 => set_image_input(&mut self.b, value),
			2 => set_value_param(&mut self.alpha, value),
			_ => false,
		}
	}

	fn resolve(
		&self,
		eval: &mut dyn EvalAccess,
		time: f64,
		ctx: &EvalContext,
	) -> Result<ResolvedOp, EvalError> {
		let a = eval.eval_image(self.a, time, ctx)?;
		let b = eval.eval_image(self.b, time, ctx)?;
		let alpha = resolve_value_param(&self.alpha, eval, time, ctx)?.as_f32()?;
		let input_hash = hash_image_pair(&a, &b);
		let param_hash = hash_f32(alpha);
		Ok(ResolvedOp::new(
			ImageMixResolved { a, b, alpha },
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
		let resolved = resolved.take::<ImageMixResolved>()?;
		if resolved.a.width != resolved.b.width || resolved.a.height != resolved.b.height {
			return Err(EvalError::ImageSizeMismatch {
				left: (resolved.a.width, resolved.a.height),
				right: (resolved.b.width, resolved.b.height),
			});
		}
		let alpha = clamp01(resolved.alpha);
		let inv_alpha = 1.0 - alpha;
		let mut rgba = Vec::with_capacity(resolved.a.rgba.len());
		for (pix_a, pix_b) in resolved.a.rgba.iter().zip(resolved.b.rgba.iter()) {
			rgba.push(pix_a * inv_alpha + pix_b * alpha);
		}
		let image = Image {
			width: resolved.a.width,
			height: resolved.a.height,
			rgba,
		};
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
