use nade_core::{CompositionId, InstanceId};

/// Tree identity for the object browser (composition outline).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectId {
	Composition(CompositionId),
	Instance {
		composition: CompositionId,
		instance: InstanceId,
	},
}

/// Messages from the object browser panel.
#[derive(Debug, Clone)]
pub enum ProjectPaneMessage {
	ToggleExpand(CompositionId),
	Select(ObjectId),
	ClearSelection,
	OpenItem(ObjectId),
	/// Create a new root composition.
	NewComposition,
	/// Add an object (instance) under the selected composition.
	AddObject,
}

/// Shell work after the panel state applied a message: selection fan-out
/// the panel cannot perform itself, and app-level requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectPaneUpdate {
	/// Fan the new selection out to the timeline, preview and curve editor.
	SelectionChanged(ObjectId),
	/// Clear the timeline selection as well.
	SelectionCleared,
	/// Create a new root composition.
	NewComposition,
	/// Add an object under the selected composition.
	AddObject,
}
