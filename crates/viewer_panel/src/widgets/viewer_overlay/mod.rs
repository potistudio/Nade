//! Viewer overlay: guides, selection gizmo and all pointer
//! input of the viewer (tools, pan, zoom). Drawn on top of the video shader.

mod drag;
mod draw;
mod hit_test;
mod layer_box;
mod program;

use crate::tool::Tool;
use crate::widgets::viewport::Viewport;
use iced::{Point, Size, Vector, keyboard};
use nade_core::{Affine, InstanceId};

pub use layer_box::LayerBox;

const SCROLL_LINE_PX: f32 = 20.0;
const WHEEL_ZOOM_SPEED: f32 = 0.01;
const CLICK_ZOOM_FACTOR: f32 = 1.5;
/// Extra hit area around layer bounds (screen px).
const HIT_SLOP: f32 = 4.0;
const HANDLE_SIZE: f32 = 6.0;
/// Grab radius of a scale handle (screen px).
const HANDLE_HIT: f32 = 6.0;
/// Scale handles in units of the half size: corners, then edge midpoints.
const HANDLES: [[f32; 2]; 8] = [
	[-1.0, -1.0],
	[1.0, -1.0],
	[1.0, 1.0],
	[-1.0, 1.0],
	[0.0, -1.0],
	[1.0, 0.0],
	[0.0, 1.0],
	[-1.0, 0.0],
];

#[derive(Debug, Clone)]
pub struct ViewerOverlay {
	pub frame: Size,
	pub zoom: f32,
	pub offset: Vector,
	pub tool: Tool,
	/// Visible layers, back to front.
	pub layers: Vec<LayerBox>,
	pub selected: Option<InstanceId>,
	pub show_grid: bool,
	pub show_safe_areas: bool,
}

#[derive(Debug, Clone, Copy)]
enum Drag {
	Pan {
		last: Point,
	},
	Move {
		last: [f32; 2],
	},
	Rotate {
		center: Point,
		last_angle: f32,
	},
	/// Scale handle drag; everything is measured against the layer as it was
	/// when the drag started.
	Scale {
		handle: [f32; 2],
		local: [f32; 2],
		inverse: Affine,
		anchor: [f32; 2],
		scale: [f32; 2],
	},
	Anchor {
		last: [f32; 2],
	},
}

#[derive(Debug, Default)]
pub struct OverlayState {
	modifiers: keyboard::Modifiers,
	drag: Option<Drag>,
	/// 最後に通知したビューアのサイズ
	reported_size: Option<Size>,
}

impl ViewerOverlay {
	fn viewport(&self, size: Size) -> Viewport {
		Viewport::new(size, self.frame, self.zoom, self.offset)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use nade_core::Transform;

	fn layer(rotation: f32) -> LayerBox {
		let transform = Transform {
			position: [100.0, 50.0, 0.0],
			rotation: [0.0, 0.0, rotation],
			..Transform::default()
		};
		LayerBox {
			id: InstanceId::new(0),
			matrix: transform.local_matrix(),
			parent: Affine::IDENTITY,
			size: [80.0, 20.0],
			anchor: [0.0, 0.0],
			scale: [1.0, 1.0],
		}
	}

	#[test]
	fn hit_testing_respects_rotation() {
		assert!(layer(0.0).contains([135.0, 50.0], 0.0));
		assert!(!layer(90.0).contains([135.0, 50.0], 0.0));
		assert!(layer(90.0).contains([100.0, 85.0], 0.0));
	}

	#[test]
	fn empty_layers_hit_near_anchor() {
		let empty = LayerBox {
			size: [0.0, 0.0],
			..layer(0.0)
		};
		assert!(empty.contains([103.0, 52.0], 4.0));
		assert!(!empty.contains([130.0, 50.0], 4.0));
	}

	#[test]
	fn moves_are_converted_into_parent_space() {
		let child = LayerBox {
			parent: Affine::rotate(90.0).then_apply(Affine::scale(2.0, 2.0)),
			..layer(0.0)
		};
		// 画面で右へ 10 → 親空間（90° 回転・2 倍）では下へ 5
		let [x, y] = ViewerOverlay::to_parent(&child, [10.0, 0.0]);
		assert!(x.abs() < 1e-4 && (y + 5.0).abs() < 1e-4, "({x}, {y})");
	}
}
