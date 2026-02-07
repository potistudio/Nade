use std::any::Any;

use crate::core::{
	EvalContext, EvalError, Image, NodeId, NodeState, Output, OutputType, Value, ValueKind,
	ValueParam,
};

pub trait EvalAccess {
	fn eval_value(
		&mut self,
		node_id: NodeId,
		time: f64,
		ctx: &EvalContext,
	) -> Result<Value, EvalError>;

	fn eval_image(
		&mut self,
		node_id: NodeId,
		time: f64,
		ctx: &EvalContext,
	) -> Result<Image, EvalError>;
}

#[derive(Debug, Clone, Copy)]
pub enum ValueParamUi {
	Default,
	Float,
	Bool,
	Color,
}

#[derive(Debug, Clone)]
pub enum ParamKind {
	ValueParam {
		expected: Option<ValueKind>,
		allow_mapping: bool,
		ui: ValueParamUi,
	},
	U32 {
		min: u32,
		max: u32,
	},
	ImageInput,
}

#[derive(Debug, Clone)]
pub enum ParamValue {
	ValueParam(ValueParam),
	U32(u32),
	ImageInput(NodeId),
}

#[derive(Debug, Clone)]
pub struct ParamDescriptor {
	pub label: String,
	pub kind: ParamKind,
	pub value: ParamValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ResolvedHashes {
	pub param_hash: u64,
	pub input_hash: u64,
}

impl ResolvedHashes {
	pub fn new(param_hash: u64, input_hash: u64) -> Self {
		Self {
			param_hash,
			input_hash,
		}
	}
}

pub struct ResolvedOp {
	hashes: ResolvedHashes,
	payload: Box<dyn Any + Send + Sync>,
}

impl ResolvedOp {
	pub fn new<T>(payload: T, param_hash: u64, input_hash: u64) -> Self
	where
		T: Any + Send + Sync,
	{
		Self {
			hashes: ResolvedHashes::new(param_hash, input_hash),
			payload: Box::new(payload),
		}
	}

	pub fn hashes(&self) -> ResolvedHashes {
		self.hashes
	}

	pub fn take<T>(self) -> Result<T, EvalError>
	where
		T: Any,
	{
		self.payload
			.downcast::<T>()
			.map(|boxed| *boxed)
			.map_err(|_| EvalError::ResolvedPayloadMismatch {
				expected: std::any::type_name::<T>(),
			})
	}
}

impl std::fmt::Debug for ResolvedOp {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ResolvedOp")
			.field("param_hash", &self.hashes.param_hash)
			.field("input_hash", &self.hashes.input_hash)
			.finish()
	}
}

pub trait Operator: std::fmt::Debug + Send + Sync + Any {
	fn name(&self) -> &'static str;
	fn output_type(&self) -> OutputType;
	fn value_kind(&self) -> Option<ValueKind> {
		None
	}
	fn parameters(&self) -> Vec<ParamDescriptor> {
		Vec::new()
	}
	fn set_parameter(&mut self, _index: usize, _value: ParamValue) -> bool {
		false
	}
	fn resolve(
		&self,
		eval: &mut dyn EvalAccess,
		time: f64,
		ctx: &EvalContext,
	) -> Result<ResolvedOp, EvalError>;
	fn compute(
		&self,
		resolved: ResolvedOp,
		time: f64,
		state_in: &NodeState,
		ctx: &EvalContext,
	) -> Result<(Output, NodeState), EvalError>;
	fn clone_box(&self) -> Box<dyn Operator>;
	fn as_any(&self) -> &dyn Any;
	fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl Clone for Box<dyn Operator> {
	fn clone(&self) -> Self {
		self.clone_box()
	}
}
