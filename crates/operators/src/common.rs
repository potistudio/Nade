use nade_core::{
	EvalAccess, EvalContext, EvalError, NodeId, ParamDescriptor, ParamKind, ParamValue, Value,
	ValueKind, ValueParam, ValueParamUi,
};

pub(crate) fn value_param_descriptor(
	label: &str,
	param: &ValueParam,
	expected: Option<ValueKind>,
	allow_mapping: bool,
	ui: ValueParamUi,
) -> ParamDescriptor {
	ParamDescriptor {
		label: label.to_string(),
		kind: ParamKind::ValueParam {
			expected,
			allow_mapping,
			ui,
		},
		value: ParamValue::ValueParam(param.clone()),
	}
}

pub(crate) fn u32_descriptor(label: &str, value: u32, min: u32, max: u32) -> ParamDescriptor {
	ParamDescriptor {
		label: label.to_string(),
		kind: ParamKind::U32 { min, max },
		value: ParamValue::U32(value),
	}
}

pub(crate) fn image_input_descriptor(label: &str, value: NodeId) -> ParamDescriptor {
	ParamDescriptor {
		label: label.to_string(),
		kind: ParamKind::ImageInput,
		value: ParamValue::ImageInput(value),
	}
}

pub(crate) fn set_value_param(target: &mut ValueParam, value: ParamValue) -> bool {
	match value {
		ParamValue::ValueParam(param) => {
			*target = param;
			true
		}
		_ => false,
	}
}

pub(crate) fn set_u32(target: &mut u32, value: ParamValue, min: u32, max: u32) -> bool {
	match value {
		ParamValue::U32(val) => {
			*target = val.clamp(min, max);
			true
		}
		_ => false,
	}
}

pub(crate) fn set_image_input(target: &mut NodeId, value: ParamValue) -> bool {
	match value {
		ParamValue::ImageInput(node_id) => {
			*target = node_id;
			true
		}
		_ => false,
	}
}

pub(crate) fn resolve_value_param(
	param: &ValueParam,
	eval: &mut dyn EvalAccess,
	time: f64,
	ctx: &EvalContext,
) -> Result<Value, EvalError> {
	match param.mapping {
		None => Ok(param.base),
		Some(node_id) => {
			let mapped = eval.eval_value(node_id, time, ctx)?;
			param.base.add_value(mapped)
		}
	}
}

pub(crate) fn clamp01(value: f32) -> f32 {
	value.clamp(0.0, 1.0)
}
