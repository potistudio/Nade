use core::{Graph, NodeId, NodeState, Operator, OutputType, ParamKind, ParamValue};
use operators::{create_operator, next_operator_key, operator_definitions, prev_operator_key};

#[derive(Debug, Default)]
pub struct Document {
	graph: Graph,
}

impl Document {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn graph(&self) -> &Graph {
		&self.graph
	}

	pub fn graph_mut(&mut self) -> &mut Graph {
		&mut self.graph
	}
}

#[derive(Debug, Default)]
pub struct EditorSession {
	document: Document,
	operator_ids: Vec<NodeId>,
}

impl EditorSession {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn document(&self) -> &Document {
		&self.document
	}

	pub fn graph(&self) -> &Graph {
		self.document.graph()
	}

	pub fn operator_ids(&self) -> &[NodeId] {
		&self.operator_ids
	}

	pub fn add_operator(&mut self) -> Option<NodeId> {
		let definition = operator_definitions().first()?;
		let operator = self.make_operator(definition.key, None)?;
		let node_id = self.document.graph_mut().add_node_boxed(operator);
		self.operator_ids.push(node_id);
		self.document.graph_mut().clear_cache();
		Some(node_id)
	}

	pub fn cycle_operator_prev(&mut self, node_id: NodeId) -> Result<(), String> {
		self.cycle_operator(node_id, false)
	}

	pub fn cycle_operator_next(&mut self, node_id: NodeId) -> Result<(), String> {
		self.cycle_operator(node_id, true)
	}

	pub fn set_operator_parameter(
		&mut self,
		node_id: NodeId,
		index: usize,
		value: ParamValue,
	) -> Result<(), String> {
		let Some(node) = self.document.graph_mut().node_mut(node_id) else {
			return Err(format!("Node {} not found", node_id.value()));
		};

		// if !node.operator.set_parameter(index, value) {
		// 	return Err(format!("Failed to set parameter {index} on node {}", node_id.value()));
		// }

		node.state = NodeState::default();
		self.document.graph_mut().clear_cache();

		Ok(())
	}

	fn cycle_operator(&mut self, node_id: NodeId, forward: bool) -> Result<(), String> {
		// let Some(current_key) = self
		// 	.document
		// 	.graph()
		// 	.node(node_id)
		// 	.map(|node| node.operator.key())
		// else {
		// 	return Err(format!("Node {} not found", node_id.value()));
		// };

		// let target_key = if forward {
		// 	next_operator_key(current_key)
		// } else {
		// 	prev_operator_key(current_key)
		// };
		// let Some(target_key) = target_key else {
		// 	return Err(format!("No target operator found for '{current_key}'"));
		// };
		// let Some(replacement) = self.make_operator(target_key, Some(node_id)) else {
		// 	return Err(format!("Failed to create operator '{target_key}'"));
		// };

		let Some(node) = self.document.graph_mut().node_mut(node_id) else {
			return Err(format!(
				"Node {} not found after modification",
				node_id.value()
			));
		};

		// node.operator = replacement;
		node.state = NodeState::default();
		self.document.graph_mut().clear_cache();

		Ok(())
	}

	fn make_operator(&self, key: &str, node_hint: Option<NodeId>) -> Option<Box<dyn Operator>> {
		let fallback = node_hint
			.or_else(|| self.operator_ids.last().copied())
			.unwrap_or_else(|| NodeId::new(1));
		let mut operator = create_operator(key)?;
		self.bind_image_inputs(operator.as_mut(), fallback);
		Some(operator)
	}

	fn bind_image_inputs(&self, operator: &mut dyn Operator, fallback: NodeId) {
		let image_nodes = self.image_node_ids();
		let mut image_input_index = 0usize;

		for (index, descriptor) in operator.parameters().into_iter().enumerate() {
			if !matches!(descriptor.kind, ParamKind::ImageInput) {
				continue;
			}

			let target = image_nodes
				.get(image_input_index)
				.copied()
				.or_else(|| image_nodes.first().copied())
				.unwrap_or(fallback);

			let _ = operator.set_parameter(index, ParamValue::ImageInput(target));
			image_input_index += 1;
		}
	}

	fn image_node_ids(&self) -> Vec<NodeId> {
		self.operator_ids
			.iter()
			.copied()
			// .filter(|node_id| {
			// 	self.document
			// 		.graph()
			// 		.node(*node_id)
			// 		.is_some_and(|node| node.operator.output_type() == OutputType::Image)
			// })
			.collect()
	}
}
