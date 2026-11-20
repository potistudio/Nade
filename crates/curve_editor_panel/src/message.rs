//! Messages and updates of the curve editor.

use core::Interpolation;
use domain::TransformProperty;
use iced::{Point, Size, mouse};

#[derive(Debug, Clone)]
pub enum CurveEditorMessage {
	SelectProperty(TransformProperty),
	AddKey,
	DeleteSelected,
	SetInterpolation(Interpolation),
	FrameAll,
	CanvasEvent(CurveCanvasEvent),
}

#[derive(Debug, Clone)]
pub enum CurveCanvasEvent {
	MousePressed {
		position: Point,
		button: mouse::Button,
		bounds: Size,
	},
	MouseReleased(mouse::Button),
	MouseMoved {
		position: Point,
		bounds: Size,
	},
	MouseWheelScrolled {
		position: Point,
		delta: mouse::ScrollDelta,
		bounds: Size,
	},
	DeletePressed,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CurveEditorUpdate {
	pub playhead_time: Option<f32>,
	pub curve_changed: bool,
}
