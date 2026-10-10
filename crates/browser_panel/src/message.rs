use core::{CompositionId, InstanceId};

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
