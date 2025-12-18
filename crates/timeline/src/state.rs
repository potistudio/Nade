//! タイムライン状態の定義

use iced::{Color, Point, Rectangle, Size, Vector};

use crate::consts::*;

// =============================================================================
// タイムラインクリップ
// =============================================================================

/// タイムラインクリップ
#[derive(Clone, Debug)]
pub struct TimelineClip {
	pub id: usize,
	pub name: String,
	pub start_time: f32,
	pub duration: f32,
	pub color: Color,
}

impl TimelineClip {
	pub fn new(id: usize, name: &str, start_time: f32, duration: f32, color: Color) -> Self {
		Self {
			id,
			name: name.to_string(),
			start_time,
			duration,
			color,
		}
	}

	#[inline]
	pub fn end_time(&self) -> f32 {
		self.start_time + self.duration
	}
}

// =============================================================================
// タイムライントラック
// =============================================================================

/// タイムライントラック
#[derive(Clone, Debug)]
pub struct TimelineTrack {
	pub name: String,
	pub clips: Vec<TimelineClip>,
	pub muted: bool,
}

impl TimelineTrack {
	pub fn new(name: &str) -> Self {
		Self {
			name: name.to_string(),
			clips: Vec::new(),
			muted: false,
		}
	}

	pub fn add_clip(&mut self, clip: TimelineClip) {
		self.clips.push(clip);
	}
}

// =============================================================================
// ドラッグ状態
// =============================================================================

#[derive(Clone, Debug, Default)]
pub(crate) enum DragState {
	#[default]
	None,
	Playhead,
	Clip {
		track_id: usize,
		clip_id: usize,
		offset: f32,
	},
	ClipResizeLeft {
		track_id: usize,
		clip_id: usize,
		original_start: f32,
		original_duration: f32,
	},
	ClipResizeRight {
		track_id: usize,
		clip_id: usize,
	},
	Panning {
		start_offset: Vector,
		start_cursor: Point,
	},
}

// =============================================================================
// レイアウト計算
// =============================================================================

#[derive(Clone, Copy, Debug)]
pub(crate) struct TimelineLayout {
	pub ruler: Rectangle,
	pub track_labels: Rectangle,
	pub content: Rectangle,
}

impl TimelineLayout {
	pub fn from_bounds(bounds: Rectangle) -> Self {
		Self {
			ruler: Rectangle {
				x: TRACK_LABEL_WIDTH,
				y: 0.0,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: RULER_HEIGHT,
			},
			track_labels: Rectangle {
				x: 0.0,
				y: RULER_HEIGHT,
				width: TRACK_LABEL_WIDTH,
				height: bounds.height - RULER_HEIGHT,
			},
			content: Rectangle {
				x: TRACK_LABEL_WIDTH,
				y: RULER_HEIGHT,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: bounds.height - RULER_HEIGHT,
			},
		}
	}

	pub fn from_bounds_absolute(bounds: Rectangle) -> Self {
		Self {
			ruler: Rectangle {
				x: bounds.x + TRACK_LABEL_WIDTH,
				y: bounds.y,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: RULER_HEIGHT,
			},
			track_labels: Rectangle {
				x: bounds.x,
				y: bounds.y + RULER_HEIGHT,
				width: TRACK_LABEL_WIDTH,
				height: bounds.height - RULER_HEIGHT,
			},
			content: Rectangle {
				x: bounds.x + TRACK_LABEL_WIDTH,
				y: bounds.y + RULER_HEIGHT,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: bounds.height - RULER_HEIGHT,
			},
		}
	}

	pub fn timeline_left(&self) -> f32 {
		self.content.x
	}
}

// =============================================================================
// タイムライン状態
// =============================================================================

/// タイムラインの状態
#[derive(Debug)]
pub struct TimelineState {
	pub tracks: Vec<TimelineTrack>,
	pub playhead_time: f32,
	pub scroll_offset: Vector,
	pub time_scale: f32,
	pub selected_clip: Option<(usize, usize)>,
	pub(crate) drag_state: DragState,
	next_clip_id: usize,
}

impl Default for TimelineState {
	fn default() -> Self {
		let mut state = Self {
			tracks: Vec::new(),
			playhead_time: 0.0,
			scroll_offset: Vector::ZERO,
			time_scale: 1.0,
			selected_clip: None,
			drag_state: DragState::None,
			next_clip_id: 0,
		};
		state.add_sample_content();
		state
	}
}

impl TimelineState {
	/// 新しいタイムライン状態を作成（空）
	pub fn new() -> Self {
		Self {
			tracks: Vec::new(),
			playhead_time: 0.0,
			scroll_offset: Vector::ZERO,
			time_scale: 1.0,
			selected_clip: None,
			drag_state: DragState::None,
			next_clip_id: 0,
		}
	}

	// -------------------------------------------------------------------------
	// 座標変換
	// -------------------------------------------------------------------------

	#[inline]
	pub(crate) fn time_to_x(&self, time: f32, timeline_left: f32) -> f32 {
		timeline_left + (time * PIXELS_PER_SECOND * self.time_scale) + self.scroll_offset.x
	}

	#[inline]
	pub(crate) fn x_to_time(&self, x: f32, timeline_left: f32) -> f32 {
		((x - timeline_left) - self.scroll_offset.x) / (PIXELS_PER_SECOND * self.time_scale)
	}

	// -------------------------------------------------------------------------
	// クリップ操作
	// -------------------------------------------------------------------------

	pub fn next_clip_id(&mut self) -> usize {
		let id = self.next_clip_id;
		self.next_clip_id += 1;
		id
	}

	pub fn add_track(&mut self, track: TimelineTrack) {
		self.tracks.push(track);
	}

	pub(crate) fn find_clip_at(&self, pos: Point, bounds: Rectangle) -> Option<(usize, usize)> {
		let timeline_left = bounds.x + TRACK_LABEL_WIDTH;
		let timeline_top = bounds.y + RULER_HEIGHT;

		for (track_index, track) in self.tracks.iter().enumerate() {
			let track_y = timeline_top + (track_index as f32 * TRACK_HEIGHT) + self.scroll_offset.y;

			if pos.y < track_y + TRACK_PADDING || pos.y > track_y + TRACK_HEIGHT - TRACK_PADDING {
				continue;
			}

			for clip in &track.clips {
				let x_start = self.time_to_x(clip.start_time, timeline_left);
				let x_end = self.time_to_x(clip.end_time(), timeline_left);

				if pos.x >= x_start && pos.x <= x_end {
					return Some((track_index, clip.id));
				}
			}
		}
		None
	}

	pub(crate) fn find_clip(&self, track_id: usize, clip_id: usize) -> Option<&TimelineClip> {
		self.tracks
			.get(track_id)
			.and_then(|t| t.clips.iter().find(|c| c.id == clip_id))
	}

	pub(crate) fn find_clip_mut(
		&mut self,
		track_id: usize,
		clip_id: usize,
	) -> Option<&mut TimelineClip> {
		self.tracks
			.get_mut(track_id)
			.and_then(|t| t.clips.iter_mut().find(|c| c.id == clip_id))
	}

	pub(crate) fn is_clip_selected(&self, track_index: usize, clip_id: usize) -> bool {
		self.selected_clip == Some((track_index, clip_id))
	}

	// -------------------------------------------------------------------------
	// スクロール
	// -------------------------------------------------------------------------

	pub(crate) fn clamp_scroll_offset(&mut self) {
		self.scroll_offset.x = self.scroll_offset.x.min(0.0);
		self.scroll_offset.y = self.scroll_offset.y.min(0.0);
	}

	pub(crate) fn apply_scroll_delta(&mut self, x: f32, y: f32) {
		self.scroll_offset.x += x;
		self.scroll_offset.y += y;
		self.clamp_scroll_offset();
	}

	// -------------------------------------------------------------------------
	// ズーム
	// -------------------------------------------------------------------------

	pub fn zoom_in(&mut self) {
		self.time_scale = (self.time_scale * 1.2).min(MAX_SCALE);
	}

	pub fn zoom_out(&mut self) {
		self.time_scale = (self.time_scale / 1.2).max(MIN_SCALE);
	}

	pub fn set_time_scale(&mut self, scale: f32) {
		self.time_scale = scale.clamp(MIN_SCALE, MAX_SCALE);
	}

	pub fn reset_zoom(&mut self) {
		self.time_scale = 1.0;
	}

	// -------------------------------------------------------------------------
	// サンプルデータ
	// -------------------------------------------------------------------------

	fn add_sample_content(&mut self) {
		let sample_clips = [
			(
				"Video",
				vec![
					("Intro", 0.0, 2.5, Color::from_rgb8(66, 133, 244)),
					("Main Scene", 3.0, 5.0, Color::from_rgb8(52, 168, 83)),
					("Outro", 9.0, 2.0, Color::from_rgb8(251, 188, 4)),
				],
			),
			(
				"Audio",
				vec![("BGM", 0.0, 11.0, Color::from_rgb8(234, 67, 53))],
			),
			(
				"Effects",
				vec![
					("Fade In", 0.0, 1.0, Color::from_rgb8(156, 39, 176)),
					("Transition", 2.5, 0.5, Color::from_rgb8(156, 39, 176)),
					("Fade Out", 10.0, 1.0, Color::from_rgb8(156, 39, 176)),
				],
			),
			(
				"Subtitles",
				vec![
					("Title", 0.5, 2.0, Color::from_rgb8(0, 188, 212)),
					("Description", 4.0, 3.0, Color::from_rgb8(0, 188, 212)),
				],
			),
		];

		for (track_name, clips) in sample_clips {
			let mut track = TimelineTrack::new(track_name);
			for (name, start, duration, color) in clips {
				track.add_clip(TimelineClip::new(
					self.next_clip_id(),
					name,
					start,
					duration,
					color,
				));
			}
			self.tracks.push(track);
		}
	}

	// -------------------------------------------------------------------------
	// ヘルパー
	// -------------------------------------------------------------------------

	#[inline]
	pub(crate) fn is_track_visible(&self, track_y: f32, rect: Rectangle) -> bool {
		track_y + TRACK_HEIGHT >= rect.y && track_y <= rect.y + rect.height
	}

	pub(crate) fn calculate_tick_interval(&self) -> f32 {
		match self.time_scale {
			s if s < 0.3 => 5.0,
			s if s < 1.0 => 2.0,
			s if s < 3.0 => 1.0,
			_ => 0.5,
		}
	}
}

// =============================================================================
// ユーティリティ
// =============================================================================

pub(crate) fn format_time(seconds: f32) -> String {
	let mins = (seconds / 60.0).floor() as i32;
	let secs = seconds % 60.0;
	if mins > 0 {
		format!("{}:{:05.2}", mins, secs)
	} else {
		format!("{:.2}s", secs)
	}
}

pub(crate) fn lighten_color(color: Color, amount: f32) -> Color {
	Color::from_rgb(
		color.r + (1.0 - color.r) * amount,
		color.g + (1.0 - color.g) * amount,
		color.b + (1.0 - color.b) * amount,
	)
}
