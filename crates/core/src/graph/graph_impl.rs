use std::collections::HashMap;

use crate::core::{EvalContext, EvalError, Image, NodeId, NodeState, Output, OutputType, Value};
use crate::graph::{CacheEntry, CacheKey, Node};
use crate::hash::context::hash_context;
use crate::ops::{EvalAccess, Operator};

#[derive(Debug)]
pub struct Graph {
	nodes: HashMap<NodeId, Node>,
	cache: HashMap<CacheKey, CacheEntry>,
	next_id: u64,
}

impl Default for Graph {
	fn default() -> Self {
		Self::new()
	}
}

impl Graph {
	pub fn new() -> Self {
		Self {
			nodes: HashMap::new(),
			cache: HashMap::new(),
			next_id: 1,
		}
	}

	pub fn add_node<O>(&mut self, operator: O) -> NodeId
	where
		O: Operator + 'static,
	{
		self.add_node_boxed(Box::new(operator))
	}

	pub fn add_node_boxed(&mut self, operator: Box<dyn Operator>) -> NodeId {
		let node_id = NodeId::new(self.next_id as usize);
		self.next_id += 1;
		self.nodes.insert(
			node_id,
			Node {
				id: node_id,
				operator,
				state: NodeState::default(),
			},
		);
		node_id
	}

	pub fn node(&self, node_id: NodeId) -> Option<&Node> {
		self.nodes.get(&node_id)
	}

	pub fn node_mut(&mut self, node_id: NodeId) -> Option<&mut Node> {
		self.nodes.get_mut(&node_id)
	}

	pub fn remove_node(&mut self, node_id: NodeId) -> Option<Node> {
		let removed = self.nodes.remove(&node_id);
		if removed.is_some() {
			self.clear_cache();
		}
		removed
	}

	pub fn clear_cache(&mut self) {
		self.cache.clear();
	}

	pub fn eval_value(
		&mut self,
		node_id: NodeId,
		time: f64,
		ctx: &EvalContext,
	) -> Result<Value, EvalError> {
		match self.eval_node(node_id, time, ctx)? {
			Output::Value(value) => Ok(value),
			Output::Image(_) => Err(EvalError::OutputTypeMismatch {
				expected: OutputType::Value,
				found: OutputType::Image,
			}),
		}
	}

	pub fn eval_image(
		&mut self,
		node_id: NodeId,
		time: f64,
		ctx: &EvalContext,
	) -> Result<Image, EvalError> {
		match self.eval_node(node_id, time, ctx)? {
			Output::Image(image) => Ok(image),
			Output::Value(_) => Err(EvalError::OutputTypeMismatch {
				expected: OutputType::Image,
				found: OutputType::Value,
			}),
		}
	}

	fn eval_node(
		&mut self,
		node_id: NodeId,
		time: f64,
		ctx: &EvalContext,
	) -> Result<Output, EvalError> {
		let (operator, state_in) = match self.nodes.get(&node_id) {
			Some(node) => (node.operator.clone(), node.state.clone()),
			None => return Err(EvalError::NodeNotFound(node_id)),
		};

		let resolved = operator.resolve(self, time, ctx)?;
		let hashes = resolved.hashes();
		let key = CacheKey::new(
			node_id,
			time,
			hashes.param_hash,
			hashes.input_hash,
			hash_context(ctx),
		);

		if let Some(entry) = self.cache.get(&key) {
			if let Some(node) = self.nodes.get_mut(&node_id) {
				node.state = entry.state.clone();
			}
			return Ok(entry.output.clone());
		}

		let (output, state_out) = operator.compute(resolved, time, &state_in, ctx)?;
		self.cache.insert(
			key,
			CacheEntry {
				output: output.clone(),
				state: state_out.clone(),
			},
		);
		if let Some(node) = self.nodes.get_mut(&node_id) {
			node.state = state_out;
		}
		Ok(output)
	}
}

impl EvalAccess for Graph {
	fn eval_value(
		&mut self,
		node_id: NodeId,
		time: f64,
		ctx: &EvalContext,
	) -> Result<Value, EvalError> {
		Graph::eval_value(self, node_id, time, ctx)
	}

	fn eval_image(
		&mut self,
		node_id: NodeId,
		time: f64,
		ctx: &EvalContext,
	) -> Result<Image, EvalError> {
		Graph::eval_image(self, node_id, time, ctx)
	}
}
