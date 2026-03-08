use crate::core::{NodeId, NodeState};
use crate::ops::Operator;

/// Node represents a processing unit in the graph, which can contain multiple operators and maintain its state.
#[derive(Debug)]
pub struct Node {
	/// ID of the node
	id: NodeId,

	/// Vector of operators in the node
	pub operator: Vec<Box<dyn Operator>>,

	pub state: NodeState,
}

/// -------- Public API --------
impl Node {
	pub fn new(id: NodeId) -> Self {
		Self {
			id,
			operator: Vec::new(),
			state: NodeState::default(),
		}
	}

	pub fn id(&self) -> NodeId {
		self.id
	}
}
