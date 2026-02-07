use crate::core::{NodeId, NodeState, Output};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
	pub node_id: NodeId,
	pub time_bits: u64,
	pub param_hash: u64,
	pub input_hash: u64,
	pub context_hash: u64,
}

impl CacheKey {
	pub fn new(
		node_id: NodeId,
		time: f64,
		param_hash: u64,
		input_hash: u64,
		context_hash: u64,
	) -> Self {
		Self {
			node_id,
			time_bits: time.to_bits(),
			param_hash,
			input_hash,
			context_hash,
		}
	}
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
	pub output: Output,
	pub state: NodeState,
}
