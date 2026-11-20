//! Undo / redo history of project snapshots.

use nade_domain::{Project, ProjectSnapshot};

/// Maximum number of undo steps kept.
const LIMIT: usize = 200;

/// Linear undo history. The app calls [`History::checkpoint`] whenever an
/// edit gesture is complete; any difference since the previous checkpoint
/// becomes one undo step.
pub struct History {
	undo: Vec<ProjectSnapshot>,
	redo: Vec<ProjectSnapshot>,
	current: ProjectSnapshot,
}

impl History {
	#[cfg(test)]
	pub fn new(project: &Project) -> Self {
		Self::with_snapshot(project.snapshot())
	}

	/// History whose initial state is `current`.
	pub fn with_snapshot(current: ProjectSnapshot) -> Self {
		Self {
			undo: Vec::new(),
			redo: Vec::new(),
			current,
		}
	}

	/// Record the project as a new step if it changed. Returns `true` when a
	/// step was recorded.
	pub fn checkpoint(&mut self, project: &Project) -> bool {
		let now = project.snapshot();
		if now == self.current {
			return false;
		}
		self.undo.push(std::mem::replace(&mut self.current, now));
		if self.undo.len() > LIMIT {
			self.undo.remove(0);
		}
		self.redo.clear();
		true
	}

	/// Step back. Returns `false` when there is nothing to undo.
	pub fn undo(&mut self, project: &mut Project) -> bool {
		self.checkpoint(project);
		let Some(previous) = self.undo.pop() else {
			return false;
		};
		self.redo.push(std::mem::replace(&mut self.current, previous.clone()));
		project.restore(previous);
		true
	}

	/// Step forward. Returns `false` when there is nothing to redo.
	pub fn redo(&mut self, project: &mut Project) -> bool {
		if self.checkpoint(project) {
			// 新しい編集があればやり直し履歴は無効
			return false;
		}
		let Some(next) = self.redo.pop() else {
			return false;
		};
		self.undo.push(std::mem::replace(&mut self.current, next.clone()));
		project.restore(next);
		true
	}

	/// Forget everything (e.g. after opening another project).
	pub fn reset(&mut self, project: &Project) {
		*self = Self::with_snapshot(project.snapshot());
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use nade_core::NodeId;

	#[test]
	fn undo_and_redo_walk_the_steps() {
		let mut project = Project::default();
		let comp = project.create_composition("Main", 640, 360, 30.0);
		let mut history = History::new(&project);

		project.composition_mut(&comp).unwrap().add_instance(NodeId::new(0));
		assert!(history.checkpoint(&project));
		assert!(!history.checkpoint(&project), "no change, no step");
		project.composition_mut(&comp).unwrap().add_instance(NodeId::new(1));

		assert!(history.undo(&mut project), "uncommitted edits are undone first");
		assert_eq!(project.composition(&comp).unwrap().len(), 1);
		assert!(history.undo(&mut project));
		assert!(project.composition(&comp).unwrap().is_empty());
		assert!(!history.undo(&mut project));

		assert!(history.redo(&mut project));
		assert!(history.redo(&mut project));
		assert_eq!(project.composition(&comp).unwrap().len(), 2);
		assert!(!history.redo(&mut project));
	}

	#[test]
	fn new_edits_clear_redo() {
		let mut project = Project::default();
		let comp = project.create_composition("Main", 640, 360, 30.0);
		let mut history = History::new(&project);
		project.composition_mut(&comp).unwrap().add_instance(NodeId::new(0));
		history.checkpoint(&project);
		history.undo(&mut project);

		project.composition_mut(&comp).unwrap().set_duration(3.0);
		assert!(!history.redo(&mut project));
		assert_eq!(project.composition(&comp).unwrap().duration(), 3.0);
	}
}
