//! Messages and updates exchanged between the timeline widget and its state.

use iced::keyboard::Key;
use iced::{Point, Rectangle, keyboard, mouse};

/// Messages that can be sent by the TimelineWidget
#[derive(Debug, Clone)]
pub enum TimelineMessage {
	ZoomIn,
	ZoomOut,
	ZoomChanged(f32),
	ResetZoom,
	PlayheadChanged(f32),
	ClipModified,
	CanvasEvent(TimelineCanvasEvent),
	ReorderTrack {
		from_index: usize,
		to_index: usize,
	},
	ToggleVisible(usize),
	ToggleSolo(usize),
	/// A keyframe diamond on a clip is being dragged: keys that sat at
	/// `origin` (when the drag began) should now sit at `time`.
	KeyframeDragged {
		clip_id: usize,
		origin: f32,
		time: f32,
	},
	/// Right-click on a clip: select it unless it is already selected.
	ContextClip {
		track_index: usize,
		clip_id: usize,
	},
}

/// Canvasから通知されるタイムライン入力イベント
#[derive(Debug, Clone)]
pub enum TimelineCanvasEvent {
	MousePressed {
		position: Point,
		bounds: Rectangle,
		modifiers: keyboard::Modifiers,
	},
	MouseReleased,
	MouseMoved {
		position: Point,
		bounds: Rectangle,
	},
	MouseWheelScrolled {
		delta: mouse::ScrollDelta,
		bounds: Rectangle,
	},
	KeyPressed {
		key: Key,
		modifiers: keyboard::Modifiers,
	},
	KeyReleased {
		key: Key,
		modifiers: keyboard::Modifiers,
	},
}

/// タイムラインメッセージ適用結果
#[derive(Debug, Clone, Copy, Default)]
pub struct TimelineUpdate {
	pub playhead_time: Option<f32>,
	pub clip_modified: bool,
	pub track_reordered: Option<(usize, usize)>,
	pub visibility_changed: bool,
}
