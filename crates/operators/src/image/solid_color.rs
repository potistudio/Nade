use crate::common::{
	resolve_value_param, set_u32, set_value_param, u32_descriptor, value_param_descriptor,
};
use crate::hash::image::hash_color_size;
use core::{
	EvalAccess, EvalContext, EvalError, Image, NodeState, Operator, Output, OutputType,
	ParamDescriptor, ParamValue, ResolvedOp, ValueKind, ValueParam, ValueParamUi,
};

#[derive(Debug, Clone)]
pub struct ImageSolidColorOp {
	pub color: ValueParam,
	pub width: u32,
	pub height: u32,
}

#[derive(Debug, Clone)]
struct ImageSolidColorResolved {
	color: [f32; 4],
	width: u32,
	height: u32,
}

impl ImageSolidColorOp {
	pub fn new(color: ValueParam, width: u32, height: u32) -> Self {
		Self {
			color,
			width,
			height,
		}
	}
}

impl Operator for ImageSolidColorOp {
	fn name(&self) -> &'static str {
		"Image.SolidColor"
	}

	fn output_type(&self) -> OutputType {
		OutputType::Image
	}

	fn parameters(&self) -> Vec<ParamDescriptor> {
		vec![
			value_param_descriptor(
				"Color",
				&self.color,
				Some(ValueKind::Vec4),
				true,
				ValueParamUi::Color,
			),
			u32_descriptor("Width", self.width, 1, 4096),
			u32_descriptor("Height", self.height, 1, 4096),
		]
	}

	fn set_parameter(&mut self, index: usize, value: ParamValue) -> bool {
		match index {
			0 => set_value_param(&mut self.color, value),
			1 => set_u32(&mut self.width, value, 1, 4096),
			2 => set_u32(&mut self.height, value, 1, 4096),
			_ => false,
		}
	}

	fn resolve(
		&self,
		eval: &mut dyn EvalAccess,
		time: f64,
		ctx: &EvalContext,
	) -> Result<ResolvedOp, EvalError> {
		let color = resolve_value_param(&self.color, eval, time, ctx)?.as_vec4()?;
		let param_hash = hash_color_size(color, self.width, self.height);
		Ok(ResolvedOp::new(
			ImageSolidColorResolved {
				color,
				width: self.width,
				height: self.height,
			},
			param_hash,
			0,
		))
	}

	fn compute(
		&self,
		resolved: ResolvedOp,
		_time: f64,
		_state_in: &NodeState,
		_ctx: &EvalContext,
	) -> Result<(Output, NodeState), EvalError> {
		let resolved = resolved.take::<ImageSolidColorResolved>()?;
		let image = Image::solid_color(resolved.color, resolved.width, resolved.height);
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
