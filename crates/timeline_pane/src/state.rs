//! タイムライン状態の定義

use iced::{Color, Point, Rectangle, Vector};

use constants::*;
use nade_core::SceneObjectId;

// =============================================================================
// タイムラインクリップ
// =============================================================================

/// タイムラインクリップ
///
/// シーンオブジェクトをタイムライン上で視覚的に表現するためのクリップです。
/// `scene_object_id` でシーンオブジェクトと紐付けられています。
#[derive(Clone, Debug)]
pub struct TimelineClip {
	/// クリップID（タイムライン内でのユニークID）
	pub id: usize,
	/// クリップ名
	pub name: String,
	/// 開始時間（秒）
	pub start_time: f32,
	/// 持続時間（秒）
	pub duration: f32,
	/// 表示色
	pub color: Color,
	/// 紐付けられたシーンオブジェクトのID
	pub scene_object_id: Option<SceneObjectId>,
}

impl TimelineClip {
	/// 新しいタイムラインクリップを作成
	pub fn new(id: usize, name: &str, start_time: f32, duration: f32, color: Color) -> Self {
		Self {
			id,
			name: name.to_string(),
			start_time,
			duration,
			color,
			scene_object_id: None,
		}
	}

	/// シーンオブジェクトに紐付けられたクリップを作成
	pub fn from_scene_object(
		id: usize,
		name: &str,
		start_time: f32,
		duration: f32,
		color: Color,
		scene_object_id: SceneObjectId,
	) -> Self {
		Self {
			id,
			name: name.to_string(),
			start_time,
			duration,
			color,
			scene_object_id: Some(scene_object_id),
		}
	}

	/// クリップの終了時間
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
	pub scroll_offset: Vector,
	pub time_scale: f32,
	pub selected_clip: Option<(usize, usize)>,
	pub(crate) drag_state: DragState,
	pub(crate) ctrl_pressed: bool,
	pub(crate) viewport_width: f32,
	next_clip_id: usize,
}

impl Default for TimelineState {
	fn default() -> Self {
		Self {
			tracks: Vec::new(),
			scroll_offset: Vector::ZERO,
			time_scale: 1.0,
			selected_clip: None,
			drag_state: DragState::None,
			ctrl_pressed: false,
			viewport_width: 800.0,
			next_clip_id: 0,
		}
	}
}

impl TimelineState {
	/// Is the playhead being dragged?
	#[inline]
	pub fn is_dragging_playhead(&self) -> bool {
		matches!(self.drag_state, DragState::Playhead)
	}

	/// 新しいタイムライン状態を作成（空）
	pub fn new() -> Self {
		Self {
			tracks: Vec::new(),
			scroll_offset: Vector::ZERO,
			time_scale: 1.0,
			selected_clip: None,
			drag_state: DragState::None,
			ctrl_pressed: false,
			viewport_width: 800.0,
			next_clip_id: 0,
		}
	}

	// -------------------------------------------------------------------------
	// 座標変換
	// -------------------------------------------------------------------------

	/// Convert a time value to an x coordinate.
	#[inline]
	pub(crate) fn time_to_x(&self, time: f32, timeline_left: f32) -> f32 {
		timeline_left + (time * PIXELS_PER_SECOND * self.time_scale) + self.scroll_offset.x
	}

	/// Convert an x coordinate to a time value.
	#[inline]
	pub(crate) fn x_to_time(&self, x: f32, timeline_left: f32) -> f32 {
		((x - timeline_left) - self.scroll_offset.x) / (PIXELS_PER_SECOND * self.time_scale)
	}

	// -------------------------------------------------------------------------
	// クリップ操作
	// -------------------------------------------------------------------------

	/// Get the next available clip ID.
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
	// Scroll
	// -------------------------------------------------------------------------

	/// Clamp the scroll offset to valid bounds
	pub fn clamp_scroll_offset(&mut self) {
		// TODO: implement basic math functions for iced::Vector
		// e.g. min(), max(), abs(), etc.
		// more: egui::math::Vec2
		self.scroll_offset.x = self.scroll_offset.x.min(0.0);
		self.scroll_offset.y = self.scroll_offset.y.min(0.0);
	}

	/// Set the position of the timeline offset
	pub fn set_scroll_position(&mut self, x: f32, y: f32) {
		self.scroll_offset.x = x;
		self.scroll_offset.y = y;
		self.clamp_scroll_offset();
	}

	/// Apply a scroll delta to the timeline offset
	/// and clamp to the minimum values
	pub fn apply_scroll_delta(&mut self, x: f32, y: f32) {
		self.scroll_offset.x += x;
		self.scroll_offset.y += y;
		self.clamp_scroll_offset();
	}

	// -------------------------------------------------------------------------
	// Zoom
	// -------------------------------------------------------------------------

	/// Update the viewport width
	pub fn update_viewport_width(&mut self, width: f32) {
		self.viewport_width = width;
	}

	/// Set the timeline zoom scale
	/// and clamp to the minimum and maximum scale
	pub fn set_zoom_scale(&mut self, scale: f32) {
		self.time_scale = scale.clamp(MIN_SCALE, MAX_SCALE);
	}

	/// Set the timeline zoom scale, keeping the playhead centered
	pub fn set_zoom_scale_centered(&mut self, scale: f32, current_time: f32) {
		let new_scale = scale.clamp(MIN_SCALE, MAX_SCALE);
		if (self.time_scale - new_scale).abs() < f32::EPSILON {
			return;
		}

		self.time_scale = new_scale;

		// Center the playhead
		// content_center_x = self.viewport_width / 2.0; (approx relative to content area start)
		// We want: (playhead_time * PPS * scale) + scroll_offset_x = content_center_x
		// So: scroll_offset_x = content_center_x - (playhead_time * PPS * scale)

		let content_center_x = (self.viewport_width - TRACK_LABEL_WIDTH) / 2.0;
		let new_offset_x = content_center_x - (current_time * PIXELS_PER_SECOND * self.time_scale);

		self.scroll_offset.x = new_offset_x;
		self.clamp_scroll_offset();
	}

	/// Zoom in the timeline by 1.2x
	/// and clamp to the maximum scale
	pub fn zoom_in(&mut self, current_time: f32) {
		self.set_zoom_scale_centered(self.time_scale * 1.2, current_time);
	}

	/// Zoom out the timeline by 1.2x
	/// and clamp to the minimum scale
	pub fn zoom_out(&mut self, current_time: f32) {
		self.set_zoom_scale_centered(self.time_scale / 1.2, current_time);
	}

	/// Reset the timeline zoom scale to 1.0
	pub fn reset_zoom(&mut self) {
		self.set_zoom_scale(1.0);
	}

	// -------------------------------------------------------------------------
	// Composition同期
	// -------------------------------------------------------------------------

	/// Compositionからタイムラインを同期
	///
	/// Composition内のすべてのシーンオブジェクトをタイムラインクリップとして追加します。
	/// 既存のクリップはクリアされます。
	pub fn sync_with_composition(&mut self, composition: &nade_core::Composition) {
		// 既存のトラックをクリア
		self.tracks.clear();
		self.next_clip_id = 0;
		self.selected_clip = None;

		// 新しいトラックを作成（オブジェクトタイプごとにまとめる）
		let mut objects_track = TimelineTrack::new("Objects");

		for obj in composition.all_objects() {
			// オブジェクトの色をRGBA -> Colorに変換
			let color = if let Some(rect) = obj.as_rectangle() {
				Color::from_rgba(
					rect.fill_color[0],
					rect.fill_color[1],
					rect.fill_color[2],
					rect.fill_color[3],
				)
			} else {
				Color::from_rgb8(100, 100, 100) // デフォルト色
			};

			let clip = TimelineClip::from_scene_object(
				self.next_clip_id(),
				obj.name(),
				obj.start_time(),
				obj.duration(),
				color,
				obj.id(),
			);

			objects_track.add_clip(clip);
		}

		// トラックをタイムラインに追加
		if !objects_track.clips.is_empty() {
			self.tracks.push(objects_track);
		}
	}

	/// クリップの変更をCompositionに反映
	///
	/// タイムライン上でクリップが移動された場合、
	/// 対応するシーンオブジェクトの開始時間を更新します。
	pub fn apply_clip_changes_to_composition(&self, composition: &mut nade_core::Composition) {
		for track in &self.tracks {
			for clip in &track.clips {
				if let Some(scene_object_id) = clip.scene_object_id
					&& let Some(obj) = composition.get_mut(scene_object_id)
				{
					// 開始時間を同期
					obj.set_start_time(clip.start_time);
					// 持続時間を同期
					obj.set_duration(clip.duration);
				}
			}
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
