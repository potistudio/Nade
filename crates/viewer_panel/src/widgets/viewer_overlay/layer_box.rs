//! A layer's box in composition space and its geometry.

use core::{Affine, InstanceId};

/// Placement of a visible layer, in comp space (origin at the comp center).
#[derive(Debug, Clone, Copy)]
pub struct LayerBox {
	pub id: InstanceId,
	/// Layer space (origin at the layer center, layer px) → comp space.
	pub matrix: Affine,
	/// Space `position` lives in (the parent's layer space) → comp space.
	pub parent: Affine,
	/// Unscaled content size (layer px). Zero for layers without content.
	pub size: [f32; 2],
	/// Anchor point relative to the layer center (layer px).
	pub anchor: [f32; 2],
	/// Current scale (1.0 = 100%).
	pub scale: [f32; 2],
}

impl LayerBox {
	pub(super) fn half(&self) -> [f32; 2] {
		[self.size[0] * 0.5, self.size[1] * 0.5]
	}

	pub(super) fn has_area(&self) -> bool {
		self.size[0] > 0.0 && self.size[1] > 0.0
	}

	pub(super) fn corners(&self) -> [[f32; 2]; 4] {
		let [hw, hh] = self.half();
		[[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]].map(|corner| self.matrix.apply(corner))
	}

	/// Handle position in layer space.
	pub(super) fn handle_local(&self, handle: [f32; 2]) -> [f32; 2] {
		let [hw, hh] = self.half();
		[handle[0] * hw, handle[1] * hh]
	}

	/// Anchor point in comp space (the layer's position on screen).
	pub(super) fn anchor_point(&self) -> [f32; 2] {
		self.matrix.apply(self.anchor)
	}

	pub(super) fn contains(&self, comp: [f32; 2], slop: f32) -> bool {
		if !self.has_area() {
			let anchor = self.anchor_point();
			return (comp[0] - anchor[0]).hypot(comp[1] - anchor[1]) <= slop * 2.0;
		}
		let Some(inverse) = self.matrix.inverse() else {
			return false;
		};
		let local = inverse.apply(comp);
		let [sx, sy] = self.matrix.axis_scales();
		let [hw, hh] = self.half();
		local[0].abs() <= hw + slop / sx.max(1e-6) && local[1].abs() <= hh + slop / sy.max(1e-6)
	}
}
