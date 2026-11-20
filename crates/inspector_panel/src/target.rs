//! What the panel shows: nothing, a composition or a layer.

use iced::Color;
use nade_core::{CompositionId, CompositionSummary, InstanceId};
use nade_domain::Instance;

pub struct LayerTarget<'a> {
	pub instance: &'a Instance,
	pub time: f32,
	pub label: Color,
	pub fps: f32,
	/// Layers this one may be parented to (no cycles), plus "None".
	pub parents: Vec<ParentChoice>,
	/// Name of the composition a comp layer plays.
	pub nested: Option<&'a str>,
}

/// An entry of the Parent picker.
#[derive(Debug, Clone, PartialEq)]
pub struct ParentChoice {
	pub id: Option<InstanceId>,
	pub name: String,
}

impl ParentChoice {
	pub fn none() -> Self {
		Self {
			id: None,
			name: "None".to_string(),
		}
	}
}

/// An entry of the "Add to" composition picker.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositionChoice {
	pub id: CompositionId,
	pub name: String,
}

impl std::fmt::Display for CompositionChoice {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.name)
	}
}

impl std::fmt::Display for ParentChoice {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.name)
	}
}

pub enum InspectorTarget<'a> {
	Empty,
	Composition {
		summary: CompositionSummary<'a>,
		layers: usize,
		background: [f32; 4],
		/// Compositions this one can be nested into (no loops).
		nest_targets: Vec<CompositionChoice>,
	},
	Layer(LayerTarget<'a>),
}
