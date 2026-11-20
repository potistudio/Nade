//! タイムラインウィジェットの実装
//!
//! After Effects 風のレイヤータイムライン: ラベル色付きのレイヤーバー、
//! キーフレームの菱形、左上のタイムコード、コンポ終端の影を描画する。

mod clips;
mod frame;
mod playhead;
mod program;
mod range_slider;
mod ruler;
mod track_labels;
mod zoom_control;

use crate::{TimelineModel, interaction::TimelineInteraction, message::TimelineMessage};
use iced::{
	Element, Length, Rectangle,
	widget::canvas::{Canvas, Path},
};
use std::collections::HashMap;

const KEYFRAME_RADIUS: f32 = 4.5;

fn squircle_rect(rect: Rectangle, radius: f32) -> Path {
	nade_constants::squircle::path(rect, radius)
}

/// Ghost of a clip about to be dropped in from outside the timeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropPreview {
	pub lane: usize,
	pub start: f32,
	pub duration: f32,
	/// A new track opens at `lane` (the lane is taken at that time, or it is
	/// past the last track), pushing the tracks below down.
	pub insert: bool,
}

/// Iced Widget for the timeline pane
/// Timeline displays tracks and clips, and allows user interaction
pub struct TimelineWidget<'a> {
	state: &'a TimelineInteraction,
	model: &'a TimelineModel,
	current_time: f32,
	reorder_preview: Option<(usize, usize)>,
	/// Track that should glow (from mouse release animation)
	glowing_track: Option<(usize, f32)>,
	/// Composition frame rate for timecode and ruler labels.
	fps: f32,
	/// Composition duration; time past it is shaded.
	duration: Option<f32>,
	/// Keyframe times per clip id (absolute composition time).
	keyframes: HashMap<usize, Vec<f32>>,
	/// Work area `[start, end]` (seconds).
	work_area: Option<[f32; 2]>,
	drop_preview: Option<DropPreview>,
}

// -------- Public API ---------------------------------------------------------
impl<'a> TimelineWidget<'a> {
	/// Create a new TimelineWidget
	pub fn new(state: &'a TimelineInteraction, model: &'a TimelineModel) -> Self {
		Self {
			state,
			model,
			current_time: 0.0,
			reorder_preview: None,
			glowing_track: state.reorder_glow,
			fps: 30.0,
			duration: None,
			keyframes: HashMap::new(),
			work_area: None,
			drop_preview: None,
		}
	}

	/// Ghost clip shown while an asset is dragged over the lanes.
	pub fn drop_preview(mut self, preview: Option<DropPreview>) -> Self {
		self.drop_preview = preview;
		self
	}

	/// Work area shown as a bar under the ruler; time outside it is dimmed.
	pub fn work_area(mut self, range: [f32; 2]) -> Self {
		self.work_area = Some(range);
		self
	}

	/// Frame rate used for the timecode readout and ruler labels.
	pub fn frame_rate(mut self, fps: f32) -> Self {
		self.fps = fps;
		self
	}

	/// Composition duration (shades the region past the end).
	pub fn duration(mut self, duration: f32) -> Self {
		self.duration = Some(duration);
		self
	}

	/// Keyframe times to mark on clips, keyed by clip id.
	pub fn keyframes(mut self, keyframes: HashMap<usize, Vec<f32>>) -> Self {
		self.keyframes = keyframes;
		self
	}

	/// Create the view element
	pub fn view(mut self, current_time: f32) -> Element<'a, TimelineMessage> {
		self.current_time = current_time;
		self.view_internal()
	}

	pub fn view_internal(self) -> Element<'a, TimelineMessage> {
		let zoom_controls = Self::zoom_control_view(self.state.time_scale);
		let timeline_canvas: Element<'a, TimelineMessage> =
			Canvas::new(self).width(Length::Fill).height(Length::Fill).into();

		iced::widget::column![timeline_canvas, zoom_controls].into()
	}
}

impl std::fmt::Debug for TimelineWidget<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("TimelineWidget").finish_non_exhaustive()
	}
}
