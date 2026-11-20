//! タイムラインインタラクション状态的定义

mod events;
mod hit_test;
mod pointer;
mod scroll;
mod zoom;

use iced::{Point, Rectangle, Vector};
use nade_constants::timeline::*;
use nade_core::{TimelineClip, TimelineModel};

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
	/// Dragging a clip with Shift held - shows ghost (original stays, drag preview moves)
	ClipDuplicating {
		track_id: usize,
		original_clip: TimelineClip,
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
	ReorderTrack {
		from_index: usize,
		current_index: usize,
	},
	/// 空白エリアをドラッグして複数クリップを範囲選択
	RangeSelect {
		start: Point,
	},
	/// レンジスライダーの左端ハンドルをドラッグ（ズームイン/アウト・左端移動）
	RangeSliderLeft {
		start_x: f32,
		init_visible_start: f32,
		fixed_visible_end: f32,
		total_duration: f32,
		slider_width: f32,
	},
	/// レンジスライダーの右端ハンドルをドラッグ（ズームイン/アウト・右端移動）
	RangeSliderRight {
		start_x: f32,
		fixed_visible_start: f32,
		init_visible_end: f32,
		total_duration: f32,
		slider_width: f32,
	},
	/// レンジスライダーの中央をドラッグ（パン）
	RangeSliderMiddle {
		start_x: f32,
		init_visible_start: f32,
		total_duration: f32,
		slider_width: f32,
	},
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TimelineLayout {
	pub range_slider: Rectangle,
	pub ruler: Rectangle,
	pub track_labels: Rectangle,
	pub content: Rectangle,
}

impl TimelineLayout {
	pub fn from_bounds(bounds: Rectangle) -> Self {
		Self {
			range_slider: Rectangle {
				x: TRACK_LABEL_WIDTH,
				y: 0.0,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: RANGE_SLIDER_HEIGHT,
			},
			ruler: Rectangle {
				x: TRACK_LABEL_WIDTH,
				y: RANGE_SLIDER_HEIGHT,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: RULER_HEIGHT,
			},
			track_labels: Rectangle {
				x: 0.0,
				y: RANGE_SLIDER_HEIGHT + RULER_HEIGHT,
				width: TRACK_LABEL_WIDTH,
				height: bounds.height - RANGE_SLIDER_HEIGHT - RULER_HEIGHT,
			},
			content: Rectangle {
				x: TRACK_LABEL_WIDTH,
				y: RANGE_SLIDER_HEIGHT + RULER_HEIGHT,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: bounds.height - RANGE_SLIDER_HEIGHT - RULER_HEIGHT,
			},
		}
	}

	pub fn from_bounds_absolute(bounds: Rectangle) -> Self {
		Self {
			range_slider: Rectangle {
				x: bounds.x + TRACK_LABEL_WIDTH,
				y: bounds.y,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: RANGE_SLIDER_HEIGHT,
			},
			ruler: Rectangle {
				x: bounds.x + TRACK_LABEL_WIDTH,
				y: bounds.y + RANGE_SLIDER_HEIGHT,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: RULER_HEIGHT,
			},
			track_labels: Rectangle {
				x: bounds.x,
				y: bounds.y + RANGE_SLIDER_HEIGHT + RULER_HEIGHT,
				width: TRACK_LABEL_WIDTH,
				height: bounds.height - RANGE_SLIDER_HEIGHT - RULER_HEIGHT,
			},
			content: Rectangle {
				x: bounds.x + TRACK_LABEL_WIDTH,
				y: bounds.y + RANGE_SLIDER_HEIGHT + RULER_HEIGHT,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: bounds.height - RANGE_SLIDER_HEIGHT - RULER_HEIGHT,
			},
		}
	}

	pub fn timeline_left(&self) -> f32 {
		self.content.x
	}
}

/// タイムラインのUIインタラクション状態
#[derive(Debug)]
pub struct TimelineInteraction {
	pub scroll_offset: Vector,
	scroll_min: Vector,
	scroll_velocity: Vector,
	scroll_target: Vector,
	scroll_bounds: Option<Rectangle>,
	scroll_gesture_active: bool,
	scroll_input_is_momentum: bool,
	last_scroll: Option<std::time::Instant>,
	last_spring_tick: Option<std::time::Instant>,
	pub time_scale: f32,
	pub selected_clip: Option<(usize, usize)>,
	pub selected_track_index: Option<usize>,
	pub(crate) drag_state: DragState,
	pub(crate) ctrl_pressed: bool,
	pub(crate) alt_pressed: bool,
	pub(crate) shift_pressed: bool,
	pub(crate) viewport_width: f32,
	/// Animation for reorder glow effect (track_index, remaining_alpha)
	pub(crate) reorder_glow: Option<(usize, f32)>,
	/// Pending reorder to be applied on mouse release
	pub(crate) pending_reorder: Option<(usize, usize)>,
	/// Currently hovered clip (track_index, clip_id)
	pub(crate) hovered_clip: Option<(usize, usize)>,
	/// 最後に記録したカーソルの絶対X座標 (ピンチズームの中心点に使用)
	pub(crate) cursor_abs_x: f32,
	/// タイムラインコンテンツエリア左端の絶対X座標 (= bounds.x + TRACK_LABEL_WIDTH)
	pub(crate) timeline_left_abs: f32,
	/// 範囲選択中の矩形 (絶対座標)
	pub(crate) selection_rect: Option<Rectangle>,
	/// 範囲選択で選択されたクリップ群 (track_index, clip_id)
	pub(crate) selected_clips: Vec<(usize, usize)>,
	/// キャンバスの左上絶対座標 (描画時のローカル変換用)
	pub(crate) canvas_origin: Point,
	/// Alt+ドラッグで縮められているクリップ (track_index, clip_id)
	pub(crate) alt_shrinking_clip: Option<(usize, usize)>,
	/// Alt+ドラッグで縮める前の元サイズ (track_index, clip_id, start_time, duration)
	pub(crate) alt_shrinking_original: Option<(usize, usize, f32, f32)>,
	/// Last seen canvas bounds (window coordinates), for drops from outside.
	pub(crate) canvas_bounds: Option<Rectangle>,
	/// Last pointer position the canvas saw over itself (window coordinates).
	pub(crate) canvas_pointer: Option<Point>,
}

/// Lane and time under a pointer dragged in from outside the timeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropSlot {
	/// Track under the pointer; `tracks.len()` means below the last track.
	pub lane: usize,
	/// Unsnapped time under the pointer, never negative.
	pub time: f32,
}

impl Default for TimelineInteraction {
	fn default() -> Self {
		Self {
			scroll_offset: Vector::ZERO,
			scroll_min: Vector::ZERO,
			scroll_velocity: Vector::ZERO,
			scroll_target: Vector::ZERO,
			scroll_bounds: None,
			scroll_gesture_active: false,
			scroll_input_is_momentum: false,
			last_scroll: None,
			last_spring_tick: None,
			time_scale: 1.0,
			selected_clip: None,
			selected_track_index: None,
			drag_state: DragState::None,
			ctrl_pressed: false,
			alt_pressed: false,
			shift_pressed: false,
			viewport_width: 800.0,
			reorder_glow: None,
			pending_reorder: None,
			hovered_clip: None,
			cursor_abs_x: TRACK_LABEL_WIDTH + 340.0,
			timeline_left_abs: TRACK_LABEL_WIDTH,
			selection_rect: None,
			selected_clips: Vec::new(),
			canvas_origin: Point::ORIGIN,
			alt_shrinking_clip: None,
			alt_shrinking_original: None,
			canvas_bounds: None,
			canvas_pointer: None,
		}
	}
}

impl TimelineInteraction {
	/// Is the playhead being dragged?
	#[inline]
	pub fn is_dragging_playhead(&self) -> bool {
		matches!(self.drag_state, DragState::Playhead)
	}

	/// 新しいタイムライン状態を作成（空）
	pub fn new() -> Self {
		Self::default()
	}

	/// 外部モデル更新後の選択状態をリセット
	pub fn clear_selection(&mut self) {
		self.selected_clip = None;
		self.selected_clips.clear();
		self.drag_state = DragState::None;
	}

	/// 単一選択、または複数選択の先頭クリップ
	pub fn primary_selection(&self) -> Option<(usize, usize)> {
		self.selected_clip.or_else(|| self.selected_clips.first().copied())
	}

	/// 選択中の全クリップ `(track_index, clip_id)`（単一選択を含む）
	pub fn selected_clip_ids(&self) -> Vec<(usize, usize)> {
		let mut clips: Vec<(usize, usize)> = self.selected_clip.into_iter().collect();
		for clip in &self.selected_clips {
			if !clips.contains(clip) {
				clips.push(*clip);
			}
		}
		clips
	}

	pub fn select_clip(&mut self, track_index: usize, clip_id: usize) {
		self.selected_clip = Some((track_index, clip_id));
		self.selected_clips.clear();
	}

	/// Where something dragged in from outside would land at `position`
	/// (window coordinates). `None` off the track lanes.
	///
	/// Only answers when the canvas itself last saw the pointer at
	/// `position`: once the pointer leaves the timeline the canvas stops
	/// reporting it, so stale bounds (a moved or closed panel) never yield a
	/// slot.
	pub fn drop_slot(&self, model: &TimelineModel, position: Point) -> Option<DropSlot> {
		if self.canvas_pointer != Some(position) {
			return None;
		}
		let layout = TimelineLayout::from_bounds_absolute(self.canvas_bounds?);
		if !layout.content.contains(position) {
			return None;
		}
		let row = ((position.y - layout.content.y - self.scroll_offset.y) / TRACK_HEIGHT).floor();
		Some(DropSlot {
			lane: (row.max(0.0) as usize).min(model.tracks.len()),
			time: self.x_to_time(position.x, layout.timeline_left()).max(0.0),
		})
	}

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
}

#[cfg(test)]
mod tests {
	use super::*;
	use nade_core::TimelineTrack;

	fn model(tracks: usize) -> TimelineModel {
		let mut model = TimelineModel::new();
		for index in 0..tracks {
			model.add_track(TimelineTrack::new(&format!("Track {index}")));
		}
		model
	}

	/// Lanes and times follow the canvas, scroll and zoom; drops below the
	/// last track ask for a new one.
	#[test]
	fn drop_slot_maps_the_pointer_to_lane_and_time() {
		let model = model(2);
		let mut state = TimelineInteraction::new();
		let canvas = Rectangle::new(Point::new(100.0, 50.0), iced::Size::new(800.0, 300.0));
		let lanes_top = canvas.y + RANGE_SLIDER_HEIGHT + RULER_HEIGHT;
		let lanes_left = canvas.x + TRACK_LABEL_WIDTH;
		state.canvas_bounds = Some(canvas);
		let slot_at = |state: &mut TimelineInteraction, point: Point| {
			state.canvas_pointer = Some(point);
			state.drop_slot(&model, point)
		};

		let slot = slot_at(
			&mut state,
			Point::new(lanes_left + PIXELS_PER_SECOND * 2.0, lanes_top + TRACK_HEIGHT * 1.5),
		)
		.unwrap();
		assert_eq!(slot.lane, 1);
		assert!((slot.time - 2.0).abs() < 1e-4);

		let below = slot_at(&mut state, Point::new(lanes_left + 1.0, lanes_top + TRACK_HEIGHT * 5.0));
		assert_eq!(below.map(|slot| slot.lane), Some(2));

		// トラック名の列やルーラーの上は対象外
		assert_eq!(slot_at(&mut state, Point::new(canvas.x + 10.0, lanes_top + 1.0)), None);
		assert_eq!(slot_at(&mut state, Point::new(lanes_left + 10.0, canvas.y + 1.0)), None);

		state.time_scale = 2.0;
		let zoomed = slot_at(
			&mut state,
			Point::new(lanes_left + PIXELS_PER_SECOND * 2.0, lanes_top + 1.0),
		)
		.unwrap();
		assert!((zoomed.time - 1.0).abs() < 1e-4);
	}

	/// A pointer the canvas has not seen (it left the timeline) never drops.
	#[test]
	fn drop_slot_ignores_pointers_the_canvas_did_not_see() {
		let model = model(1);
		let mut state = TimelineInteraction::new();
		let canvas = Rectangle::new(Point::ORIGIN, iced::Size::new(800.0, 300.0));
		let inside = Point::new(TRACK_LABEL_WIDTH + 50.0, RANGE_SLIDER_HEIGHT + RULER_HEIGHT + 5.0);
		state.canvas_bounds = Some(canvas);

		assert_eq!(state.drop_slot(&model, inside), None);
		state.canvas_pointer = Some(inside);
		assert!(state.drop_slot(&model, inside).is_some());
		assert_eq!(state.drop_slot(&model, Point::new(inside.x + 1.0, inside.y)), None);
	}
}
