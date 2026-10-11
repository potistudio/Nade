//! Workspace presets: whole-window panel layouts tuned for a task.

use panel_system::{DockNode, LayoutBuilder};

use crate::panel_content::PanelContent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Workspace {
	/// Layers · Viewer/Timeline · Properties
	#[default]
	Animate,
	/// Viewer over a large curve editor
	Graph,
	/// Media-heavy layout with a tall timeline
	Edit,
}

impl Workspace {
	pub const ALL: [Self; 3] = [Self::Animate, Self::Graph, Self::Edit];

	pub fn index(self) -> usize {
		Self::ALL.iter().position(|workspace| *workspace == self).unwrap_or(0)
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::Animate => "Animate",
			Self::Graph => "Graph",
			Self::Edit => "Edit",
		}
	}

	/// Panel layout for this workspace and the next free area id.
	///
	/// ```text
	/// Animate                         Graph                           Edit
	/// ┌──────┬────────────┬──────┐    ┌──────┬────────────┬──────┐    ┌──────┬────────────┬──────┐
	/// │Layers│   Viewer   │Props │    │Layers│   Viewer   │Props │    │Assets│   Viewer   │Props │
	/// ├──────┼────────────┤      │    │      ├────────────┤      │    │      ├────────────┤      │
	/// │Assets│  Timeline  │      │    │      │   Curves   │      │    │Layers│  Timeline  │      │
	/// └──────┴────────────┴──────┘    └──────┴────────────┴──────┘    └──────┴────────────┴──────┘
	/// ```
	pub fn layout(self) -> (DockNode<PanelContent>, usize) {
		let mut builder = LayoutBuilder::new();
		let layout = match self {
			Self::Animate => {
				let layers = builder.area(PanelContent::Project);
				let assets = builder.area(PanelContent::Assets);
				let viewer = builder.area(PanelContent::MainPreview);
				let timeline = builder.area(PanelContent::Timeline);
				let properties = builder.area(PanelContent::Inspector);

				let left = LayoutBuilder::vsplit(layers, assets, 0.58);
				let center = LayoutBuilder::vsplit(viewer, timeline, 0.62);
				let main = LayoutBuilder::hsplit(center, properties, 0.7);
				LayoutBuilder::hsplit(left, main, 0.17)
			}
			Self::Graph => {
				let layers = builder.area(PanelContent::Project);
				let viewer = builder.area(PanelContent::MainPreview);
				let curves = builder.area(PanelContent::CurveEditor);
				let properties = builder.area(PanelContent::Inspector);

				let center = LayoutBuilder::vsplit(viewer, curves, 0.5);
				let main = LayoutBuilder::hsplit(center, properties, 0.7);
				LayoutBuilder::hsplit(layers, main, 0.17)
			}
			Self::Edit => {
				let assets = builder.area(PanelContent::Assets);
				let layers = builder.area(PanelContent::Project);
				let viewer = builder.area(PanelContent::MainPreview);
				let timeline = builder.area(PanelContent::Timeline);
				let properties = builder.area(PanelContent::Inspector);

				let left = LayoutBuilder::vsplit(assets, layers, 0.55);
				let center = LayoutBuilder::vsplit(viewer, timeline, 0.52);
				let main = LayoutBuilder::hsplit(center, properties, 0.7);
				LayoutBuilder::hsplit(left, main, 0.18)
			}
		};
		(layout, builder.next_area_id())
	}
}
