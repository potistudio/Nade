//! Layer, handle and cursor hit testing.

use crate::widgets::viewer_overlay::layer_box::LayerBox;
use crate::widgets::viewer_overlay::{HANDLE_HIT, HANDLES, HIT_SLOP, ViewerOverlay};
use crate::widgets::viewport::Viewport;
use iced::{Point, mouse};

impl ViewerOverlay {
	/// Topmost layer under `point` (screen space).
	pub(super) fn hit_test(&self, viewport: &Viewport, point: Point) -> Option<&LayerBox> {
		let comp = viewport.screen_to_comp(point);
		let slop = HIT_SLOP / viewport.display_scale().max(f32::EPSILON);
		// 選択中のレイヤーを優先し、次に前面から探す
		self.layers
			.iter()
			.filter(|layer| Some(layer.id) == self.selected)
			.chain(self.layers.iter().rev())
			.find(|layer| layer.contains(comp, slop))
	}

	pub(super) fn selected_layer(&self) -> Option<&LayerBox> {
		self.layers.iter().find(|layer| Some(layer.id) == self.selected)
	}

	/// Scale handle of the selected layer under `point` (screen space).
	pub(super) fn hit_handle(&self, viewport: &Viewport, point: Point) -> Option<(&LayerBox, [f32; 2])> {
		let layer = self.selected_layer().filter(|layer| layer.has_area())?;
		HANDLES
			.into_iter()
			.map(|handle| (layer, handle))
			.find(|(layer, handle)| {
				let screen = viewport.comp_to_screen(layer.matrix.apply(layer.handle_local(*handle)));
				(screen.x - point.x).hypot(screen.y - point.y) <= HANDLE_HIT
			})
	}
}

impl ViewerOverlay {
	pub(super) fn handle_cursor(handle: [f32; 2]) -> mouse::Interaction {
		match handle {
			[0.0, _] => mouse::Interaction::ResizingVertically,
			[_, 0.0] => mouse::Interaction::ResizingHorizontally,
			[x, y] if x * y > 0.0 => mouse::Interaction::ResizingDiagonallyDown,
			_ => mouse::Interaction::ResizingDiagonallyUp,
		}
	}
}
