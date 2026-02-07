use crate::core::{NodeId, NodeState};
use crate::ops::Operator;

#[derive(Debug, Clone)]
pub struct Node {
	pub id: NodeId,
	pub operator: Box<dyn Operator>,
	pub state: NodeState,
}
