use std::any::Any;

use super::{Image, Value};

pub trait OperatorState: std::fmt::Debug + Send + Sync {
	fn clone_box(&self) -> Box<dyn OperatorState>;
	fn as_any(&self) -> &dyn Any;
	fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> OperatorState for T
where
	T: 'static + Clone + std::fmt::Debug + Send + Sync,
{
	fn clone_box(&self) -> Box<dyn OperatorState> {
		Box::new(self.clone())
	}

	fn as_any(&self) -> &dyn Any {
		self
	}

	fn as_any_mut(&mut self) -> &mut dyn Any {
		self
	}
}

impl Clone for Box<dyn OperatorState> {
	fn clone(&self) -> Self {
		self.clone_box()
	}
}

#[derive(Debug, Clone)]
pub enum NodeState {
	Empty,
	DelayValue {
		previous: Option<Value>,
		previous_time: Option<f64>,
	},
	DelayImage {
		previous: Option<Image>,
		previous_time: Option<f64>,
	},
	Custom(Box<dyn OperatorState>),
}

impl Default for NodeState {
	fn default() -> Self {
		Self::Empty
	}
}
