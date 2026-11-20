use crate::{ObjectId, ProjectPaneMessage, ProjectPaneUpdate};
use nade_core::CompositionId;
use nade_domain::Project;
use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct ProjectPaneState {
	pub expanded_ids: HashSet<CompositionId>,
	pub selected_id: Option<ObjectId>,
}

// -------- Public API ---------------------------------------------------------
impl ProjectPaneState {
	/// Applies panel-local messages to the view state and returns the
	/// shell work that remains (selection fan-out, app-level requests).
	pub fn apply(&mut self, project: &Project, msg: ProjectPaneMessage) -> Option<ProjectPaneUpdate> {
		match msg {
			ProjectPaneMessage::ToggleExpand(id) => {
				if self.expanded_ids.contains(&id) {
					self.expanded_ids.remove(&id);
				} else {
					self.expanded_ids.insert(id);
				}
				None
			}
			ProjectPaneMessage::Select(id) => {
				self.selected_id = Some(id);
				Some(ProjectPaneUpdate::SelectionChanged(id))
			}
			ProjectPaneMessage::ClearSelection => {
				self.selected_id = None;
				Some(ProjectPaneUpdate::SelectionCleared)
			}
			ProjectPaneMessage::OpenItem(id) => Some(ProjectPaneUpdate::SelectionChanged(self.open(project, id))),
			ProjectPaneMessage::NewComposition => Some(ProjectPaneUpdate::NewComposition),
			ProjectPaneMessage::AddObject => Some(ProjectPaneUpdate::AddObject),
		}
	}

	/// Opens a tree item: comp layers resolve to the composition they play
	/// (AE-style double-click); compositions additionally expand.
	///
	/// Returns the effective selection target.
	pub fn open(&mut self, project: &Project, id: ObjectId) -> ObjectId {
		// コンポレイヤーのダブルクリックは中身のコンポを開く（AE と同じ）
		if let ObjectId::Instance { composition, instance } = id
			&& let Some(nested) = project
				.composition(&composition)
				.and_then(|comp| comp.get(instance))
				.and_then(|layer| layer.content().as_composition())
			&& project.composition(&nested).is_some()
		{
			return self.open(project, ObjectId::Composition(nested));
		}
		self.selected_id = Some(id);
		if let ObjectId::Composition(composition_id) = id {
			self.expanded_ids.insert(composition_id);
		}
		id
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use nade_core::{BitDepth, SampleRate};
	use nade_domain::InstanceContent;

	fn project() -> Project {
		Project::new("Test", BitDepth::F32, SampleRate::Hz48000)
	}

	/// Outer composition holding a layer that plays `inner`.
	fn nested_project() -> (Project, CompositionId, ObjectId) {
		let mut project = project();
		let outer = project.create_composition("Outer", 1920, 1080, 60.0);
		let inner = project.create_composition("Inner", 1920, 1080, 60.0);
		let node = project.create_node();
		let layer = {
			let comp = project.composition_mut(&outer).unwrap();
			let instance = comp.add_instance_named(node, "Inner Layer");
			instance.set_content(InstanceContent::Composition(inner));
			instance.id()
		};
		let layer = ObjectId::Instance {
			composition: outer,
			instance: layer,
		};
		(project, inner, layer)
	}

	/// Toggling disclosure twice collapses the composition again.
	#[test]
	fn toggle_expand_is_reversible() {
		let mut project = project();
		let id = project.create_composition("Comp", 1920, 1080, 60.0);
		let mut state = ProjectPaneState::default();

		state.apply(&project, ProjectPaneMessage::ToggleExpand(id));
		assert!(state.expanded_ids.contains(&id));

		state.apply(&project, ProjectPaneMessage::ToggleExpand(id));
		assert!(!state.expanded_ids.contains(&id));
	}

	/// Selecting reports the selection for the shell to fan out.
	#[test]
	fn select_reports_the_selection() {
		let mut project = project();
		let id = ObjectId::Composition(project.create_composition("Comp", 1920, 1080, 60.0));
		let mut state = ProjectPaneState::default();

		assert_eq!(
			state.apply(&project, ProjectPaneMessage::Select(id)),
			Some(ProjectPaneUpdate::SelectionChanged(id))
		);
		assert_eq!(state.selected_id, Some(id));
	}

	/// Clearing empties the selection and reports it.
	#[test]
	fn clear_selection_empties_and_reports() {
		let mut project = project();
		let id = ObjectId::Composition(project.create_composition("Comp", 1920, 1080, 60.0));
		let mut state = ProjectPaneState::default();
		state.apply(&project, ProjectPaneMessage::Select(id));

		assert_eq!(
			state.apply(&project, ProjectPaneMessage::ClearSelection),
			Some(ProjectPaneUpdate::SelectionCleared)
		);
		assert_eq!(state.selected_id, None);
	}

	/// Opening a composition selects and expands it.
	#[test]
	fn open_item_expands_compositions() {
		let mut project = project();
		let comp = project.create_composition("Comp", 1920, 1080, 60.0);
		let id = ObjectId::Composition(comp);
		let mut state = ProjectPaneState::default();

		assert_eq!(
			state.apply(&project, ProjectPaneMessage::OpenItem(id)),
			Some(ProjectPaneUpdate::SelectionChanged(id))
		);
		assert_eq!(state.selected_id, Some(id));
		assert!(state.expanded_ids.contains(&comp));
	}

	/// Opening a comp layer resolves to the composition it plays (AE-style).
	#[test]
	fn open_item_resolves_nested_compositions() {
		let (project, inner, layer) = nested_project();
		let target = ObjectId::Composition(inner);
		let mut state = ProjectPaneState::default();

		assert_eq!(
			state.apply(&project, ProjectPaneMessage::OpenItem(layer)),
			Some(ProjectPaneUpdate::SelectionChanged(target))
		);
		assert_eq!(state.selected_id, Some(target));
		assert!(state.expanded_ids.contains(&inner));
	}

	/// App-level messages come back for the shell.
	#[test]
	fn app_level_messages_pass_through() {
		let project = project();
		let mut state = ProjectPaneState::default();

		assert_eq!(
			state.apply(&project, ProjectPaneMessage::NewComposition),
			Some(ProjectPaneUpdate::NewComposition)
		);
		assert_eq!(
			state.apply(&project, ProjectPaneMessage::AddObject),
			Some(ProjectPaneUpdate::AddObject)
		);
	}
}
