//! タイムラインインタラクション状态的定义

use iced::{
	Point, Rectangle, Vector,
	keyboard::{self, Key},
	mouse,
};

use constants::timeline::*;
use core::{TimelineClip, TimelineModel};

use crate::widget::{TimelineCanvasEvent, TimelineMessage, TimelineUpdate};

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
		clip_id: usize,
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
}

impl Default for TimelineInteraction {
	fn default() -> Self {
		Self {
			scroll_offset: Vector::ZERO,
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
		self.drag_state = DragState::None;
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

	fn find_clips_in_rect(&self, model: &TimelineModel, sel_rect: Rectangle, bounds: Rectangle) -> Vec<(usize, usize)> {
		let timeline_left = bounds.x + TRACK_LABEL_WIDTH;
		let timeline_top = bounds.y + RANGE_SLIDER_HEIGHT + RULER_HEIGHT;
		let mut result = Vec::new();

		for (track_index, track) in model.tracks.iter().enumerate() {
			let track_y = timeline_top + (track_index as f32 * TRACK_HEIGHT) + self.scroll_offset.y;
			let clip_top = track_y + TRACK_PADDING;
			let clip_bottom = track_y + TRACK_HEIGHT - TRACK_PADDING;

			if sel_rect.y + sel_rect.height < clip_top || sel_rect.y > clip_bottom {
				continue;
			}

			for clip in &track.clips {
				let x_start = self.time_to_x(clip.start_time, timeline_left);
				let x_end = self.time_to_x(clip.end_time(), timeline_left);

				if sel_rect.x < x_end && sel_rect.x + sel_rect.width > x_start {
					result.push((track_index, clip.id));
				}
			}
		}

		result
	}

	pub(crate) fn find_clip_at(&self, model: &TimelineModel, pos: Point, bounds: Rectangle) -> Option<(usize, usize)> {
		let timeline_left = bounds.x + TRACK_LABEL_WIDTH;
		let timeline_top = bounds.y + RANGE_SLIDER_HEIGHT + RULER_HEIGHT;

		for (track_index, track) in model.tracks.iter().enumerate() {
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

	pub(crate) fn find_clip(model: &TimelineModel, track_id: usize, clip_id: usize) -> Option<&TimelineClip> {
		model
			.tracks
			.get(track_id)
			.and_then(|track| track.clips.iter().find(|clip| clip.id == clip_id))
	}

	pub(crate) fn find_clip_mut(
		model: &mut TimelineModel,
		track_id: usize,
		clip_id: usize,
	) -> Option<&mut TimelineClip> {
		model
			.tracks
			.get_mut(track_id)
			.and_then(|track| track.clips.iter_mut().find(|clip| clip.id == clip_id))
	}

	pub(crate) fn is_clip_selected(&self, track_index: usize, clip_id: usize) -> bool {
		self.selected_clip == Some((track_index, clip_id)) || self.selected_clips.contains(&(track_index, clip_id))
	}

	pub(crate) fn is_clip_hovered(&self, track_index: usize, clip_id: usize) -> bool {
		self.hovered_clip == Some((track_index, clip_id))
	}

	/// Clamp the scroll offset to valid bounds
	pub fn clamp_scroll_offset(&mut self) {
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

	/// Update the viewport width
	pub fn update_viewport_width(&mut self, width: f32) {
		self.viewport_width = width;
	}

	/// Set the timeline zoom scale
	/// and clamp to the minimum and maximum scale
	pub fn set_zoom_scale(&mut self, scale: f32) {
		self.time_scale = scale.clamp(MIN_SCALE, MAX_SCALE);
	}

	/// ズームをカーソル位置中心で行う
	///
	/// `factor` = 1.0 より大きければズームイン、小さければズームアウト。
	/// カーソル下の時間位置が画面上で動かないようにスクロールオフセットを調整する。
	pub fn zoom_at_cursor_x(&mut self, factor: f32) {
		let new_scale = (self.time_scale * factor).clamp(MIN_SCALE, MAX_SCALE);
		if (self.time_scale - new_scale).abs() < f32::EPSILON {
			return;
		}

		let cursor = self.cursor_abs_x;
		let tl = self.timeline_left_abs;

		let time_at_cursor = (cursor - tl - self.scroll_offset.x) / (PIXELS_PER_SECOND * self.time_scale);
		self.time_scale = new_scale;
		self.scroll_offset.x = cursor - tl - time_at_cursor * PIXELS_PER_SECOND * new_scale;
		self.clamp_scroll_offset();
	}

	/// Set the timeline zoom scale, keeping the playhead centered
	pub fn set_zoom_scale_centered(&mut self, scale: f32, current_time: f32) {
		let new_scale = scale.clamp(MIN_SCALE, MAX_SCALE);
		if (self.time_scale - new_scale).abs() < f32::EPSILON {
			return;
		}

		self.time_scale = new_scale;

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

	/// 現在のビューポートで見えている先頭の時間
	pub fn visible_start_time(&self) -> f32 {
		(-self.scroll_offset.x / (PIXELS_PER_SECOND * self.time_scale)).max(0.0)
	}

	/// 現在のビューポートで見えている末尾の時間
	pub fn visible_end_time(&self) -> f32 {
		let content_w = (self.viewport_width - TRACK_LABEL_WIDTH).max(0.0);
		self.visible_start_time() + content_w / (PIXELS_PER_SECOND * self.time_scale)
	}

	/// タイムライン全体の長さ（クリップの末尾と表示末尾の大きい方、最低30秒）
	pub fn total_timeline_duration(&self, model: &TimelineModel) -> f32 {
		let max_clip = model
			.tracks
			.iter()
			.flat_map(|t| t.clips.iter())
			.map(|c| c.end_time())
			.fold(0.0f32, f32::max);
		max_clip.max(self.visible_end_time()).max(30.0)
	}
}

impl TimelineInteraction {
	/// タイムラインメッセージを状態に適用
	pub fn apply_message(
		&mut self,
		model: &mut TimelineModel,
		message: TimelineMessage,
		current_time: f32,
	) -> TimelineUpdate {
		match message {
			TimelineMessage::ZoomIn => {
				self.zoom_in(current_time);
				TimelineUpdate::default()
			}
			TimelineMessage::ZoomOut => {
				self.zoom_out(current_time);
				TimelineUpdate::default()
			}
			TimelineMessage::ZoomChanged(scale) => {
				self.set_zoom_scale_centered(scale, current_time);
				TimelineUpdate::default()
			}
			TimelineMessage::ResetZoom => {
				self.reset_zoom();
				TimelineUpdate::default()
			}
			TimelineMessage::PlayheadChanged(time) => TimelineUpdate {
				playhead_time: Some(time),
				clip_modified: false,
				track_reordered: None,
			},
			TimelineMessage::ClipModified => TimelineUpdate {
				playhead_time: None,
				clip_modified: true,
				track_reordered: None,
			},
			TimelineMessage::CanvasEvent(event) => self.handle_canvas_event(model, event, current_time),
			TimelineMessage::ReorderTrack { .. } => {
				// Handled via pending_reorder in handle_mouse_release
				TimelineUpdate::default()
			}
		}
	}

	fn handle_canvas_event(
		&mut self,
		model: &mut TimelineModel,
		event: TimelineCanvasEvent,
		current_time: f32,
	) -> TimelineUpdate {
		match event {
			TimelineCanvasEvent::MousePressed {
				position,
				bounds,
				modifiers,
			} => {
				self.update_viewport_width(bounds.width);
				self.ctrl_pressed = modifiers.command();
				self.shift_pressed = modifiers.shift();
				self.alt_pressed = modifiers.alt();
				let (_handled, playhead_time) = self.handle_mouse_press(model, position, bounds);
				TimelineUpdate {
					playhead_time,
					clip_modified: false,
					track_reordered: None,
				}
			}
			TimelineCanvasEvent::MouseReleased => {
				let was_dragging_clip = self.is_dragging_clip();
				let was_duplicating = self.is_duplicating_clip();
				let (_handled, playhead_time) = self.handle_mouse_release(model);
				TimelineUpdate {
					playhead_time,
					clip_modified: was_dragging_clip || was_duplicating,
					track_reordered: None,
				}
			}
			TimelineCanvasEvent::MouseMoved { position, bounds } => {
				self.update_viewport_width(bounds.width);
				let timeline_left = TimelineLayout::from_bounds_absolute(bounds).timeline_left();
				let (handled, playhead_time) = self.handle_mouse_move(model, position, timeline_left, bounds);
				TimelineUpdate {
					playhead_time,
					clip_modified: handled && self.is_dragging_clip(),
					track_reordered: None,
				}
			}
			TimelineCanvasEvent::MouseWheelScrolled { delta, bounds } => {
				self.update_viewport_width(bounds.width);
				self.handle_scroll(delta, current_time);
				TimelineUpdate::default()
			}
			TimelineCanvasEvent::KeyPressed { key, modifiers } => {
				let is_alt_key = matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Alt));
				self.update_modifiers(modifiers);
				self.shift_pressed = modifiers.shift();
				if is_alt_key {
					self.alt_pressed = true;
				}
				self.handle_keyboard(key, modifiers, current_time);
				TimelineUpdate::default()
			}
			TimelineCanvasEvent::KeyReleased { key, modifiers } => {
				let is_alt_key = matches!(key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Alt));
				if is_alt_key {
					self.alt_pressed = false;
				}
				self.shift_pressed = modifiers.shift();
				self.update_modifiers(modifiers);
				TimelineUpdate::default()
			}
		}
	}

	/// Is a clip being dragged?
	#[inline]
	pub fn is_dragging_clip(&self) -> bool {
		matches!(
			self.drag_state,
			DragState::Clip { .. }
				| DragState::ClipDuplicating { .. }
				| DragState::ClipResizeLeft { .. }
				| DragState::ClipResizeRight { .. }
		)
	}

	/// Is a clip being duplicated (shift+drag)?
	#[inline]
	pub fn is_duplicating_clip(&self) -> bool {
		matches!(self.drag_state, DragState::ClipDuplicating { .. })
	}

	/// マウスプレス時の処理
	///
	/// プレイヘッドドラッグ開始時は開始時間を返す
	/// Returns (handled, playhead_time)
	pub(crate) fn handle_mouse_press(
		&mut self,
		model: &TimelineModel,
		pos: Point,
		bounds: Rectangle,
	) -> (bool, Option<f32>) {
		let layout = TimelineLayout::from_bounds_absolute(bounds);

		// Check if clicking on the range slider strip
		if layout.range_slider.contains(pos) {
			let total_duration = self.total_timeline_duration(model);
			let padding = 3.0_f32;
			let edge_hit = 8.0_f32;
			let track_left = layout.range_slider.x + padding;
			let track_width = (layout.range_slider.width - padding * 2.0).max(1.0);

			let visible_start = self.visible_start_time();
			let visible_end = self.visible_end_time();

			let handle_l = track_left + (visible_start / total_duration).clamp(0.0, 1.0) * track_width;
			let handle_r =
				(track_left + (visible_end / total_duration).clamp(0.0, 1.0) * track_width).max(handle_l + 4.0);

			self.drag_state = if pos.x < handle_l + edge_hit {
				DragState::RangeSliderLeft {
					start_x: pos.x,
					init_visible_start: visible_start,
					fixed_visible_end: visible_end,
					total_duration,
					slider_width: track_width,
				}
			} else if pos.x > handle_r - edge_hit {
				DragState::RangeSliderRight {
					start_x: pos.x,
					fixed_visible_start: visible_start,
					init_visible_end: visible_end,
					total_duration,
					slider_width: track_width,
				}
			} else {
				DragState::RangeSliderMiddle {
					start_x: pos.x,
					init_visible_start: visible_start,
					total_duration,
					slider_width: track_width,
				}
			};
			return (true, None);
		}

		// Check if clicking on track label area for reordering
		if layout.track_labels.contains(pos) {
			let track_labels_top = layout.track_labels.y;
			let track_index = ((pos.y - track_labels_top - self.scroll_offset.y) / TRACK_HEIGHT).floor() as usize;

			if track_index < model.tracks.len() {
				let track_left = bounds.x;
				let handle_right = track_left + 8.0; // Handle width is 8px

				// Check if click is in the reorder handle area (left side)
				if pos.x >= track_left && pos.x <= handle_right {
					log::info!("Track reorder started: track_index={}", track_index);
					self.selected_track_index = Some(track_index);
					// from_index is the gap position (same as track index when not moved yet)
					self.drag_state = DragState::ReorderTrack {
						from_index: track_index,
						current_index: track_index,
					};
					return (true, None);
				}
			}

			// Otherwise just select the track
			self.selected_track_index = Some(track_index);
			return (true, None);
		}

		if layout.ruler.contains(pos) {
			let time = self.x_to_time(pos.x, layout.timeline_left()).max(0.0);
			self.drag_state = DragState::Playhead;
			return (true, Some(time));
		}

		if layout.content.contains(pos) {
			if let Some((track_id, clip_id)) = self.find_clip_at(model, pos, bounds) {
				if self.ctrl_pressed {
					// Cmd/Ctrl: 既存の単一選択を selected_clips に吸収してからトグル
					if let Some(existing) = self.selected_clip.take() {
						if !self.selected_clips.contains(&existing) {
							self.selected_clips.push(existing);
						}
					}
					let pair = (track_id, clip_id);
					if let Some(idx) = self.selected_clips.iter().position(|&c| c == pair) {
						self.selected_clips.remove(idx);
					} else {
						self.selected_clips.push(pair);
					}
					return (true, None);
				}

				// 複数選択に含まれていないクリップをクリック → 単一選択に切り替え
				if !self.selected_clips.contains(&(track_id, clip_id)) {
					self.selected_clip = Some((track_id, clip_id));
					self.selected_clips.clear();
				}
				self.start_clip_drag(model, pos, track_id, clip_id, layout.timeline_left());
				return (true, None);
			}

			if !self.ctrl_pressed {
				self.selected_clip = None;
				self.selected_clips.clear();
			}
			self.canvas_origin = bounds.position();
			self.drag_state = DragState::RangeSelect { start: pos };
			return (true, None);
		}

		(false, None)
	}

	fn start_clip_drag(
		&mut self,
		model: &TimelineModel,
		pos: Point,
		track_id: usize,
		clip_id: usize,
		timeline_left: f32,
	) {
		if let Some(clip) = Self::find_clip(model, track_id, clip_id) {
			let x_start = self.time_to_x(clip.start_time, timeline_left);
			let x_end = self.time_to_x(clip.end_time(), timeline_left);

			// Resize handles don't support duplicating
			if pos.x < x_start + RESIZE_HANDLE_WIDTH {
				self.drag_state = DragState::ClipResizeLeft {
					track_id,
					clip_id,
					original_start: clip.start_time,
					original_duration: clip.duration,
				};
			} else if pos.x > x_end - RESIZE_HANDLE_WIDTH {
				self.drag_state = DragState::ClipResizeRight { track_id, clip_id };
			} else if self.shift_pressed {
				// Shift+drag: duplicate mode - store the original clip for ghost rendering
				let time = self.x_to_time(pos.x, timeline_left);
				self.drag_state = DragState::ClipDuplicating {
					track_id,
					clip_id,
					original_clip: clip.clone(),
					offset: clip.start_time - time,
				};
			} else {
				let time = self.x_to_time(pos.x, timeline_left);
				self.drag_state = DragState::Clip {
					track_id,
					clip_id,
					offset: clip.start_time - time,
				};
			}
		}
	}

	pub(crate) fn handle_mouse_move(
		&mut self,
		model: &mut TimelineModel,
		pos: Point,
		timeline_left: f32,
		bounds: Rectangle,
	) -> (bool, Option<f32>) {
		self.cursor_abs_x = pos.x;
		self.timeline_left_abs = timeline_left;
		match self.drag_state.clone() {
			DragState::Playhead => {
				let time = self.x_to_time(pos.x, timeline_left).max(0.0);
				(true, Some(time))
			}
			DragState::Clip {
				track_id,
				clip_id,
				offset,
			} => {
				// 複数選択中は全クリップをまとめて移動（衝突スナップ付き）
				if self.selected_clips.len() > 1 {
					let cursor_time = self.x_to_time(pos.x, timeline_left);
					let proposed = (cursor_time + offset).max(0.0);

					let current_start = model
						.tracks
						.get(track_id)
						.and_then(|t| t.clips.iter().find(|c| c.id == clip_id))
						.map(|c| c.start_time)
						.unwrap_or(0.0);

					let raw_delta = proposed - current_start;
					let selected = self.selected_clips.clone();

					// 各選択クリップと同トラックの非選択クリップとの衝突でdeltaを絞り込む
					let mut constrained_delta = raw_delta;
					for &(sel_track, sel_clip_id) in &selected {
						let Some(track) = model.tracks.get(sel_track) else {
							continue;
						};
						let Some(sel) = track.clips.iter().find(|c| c.id == sel_clip_id) else {
							continue;
						};
						let sel_start = sel.start_time;
						let sel_end = sel_start + sel.duration;

						for blocker in track.clips.iter().filter(|c| !selected.contains(&(sel_track, c.id))) {
							let b_start = blocker.start_time;
							let b_end = b_start + blocker.duration;

							if raw_delta > 0.0 {
								// 右移動: sel の右端が blocker と重なりそうなら手前でスナップ
								if sel_end + constrained_delta > b_start && sel_start + constrained_delta < b_end {
									constrained_delta = constrained_delta.min((b_start - sel_end).max(0.0));
								}
							} else if raw_delta < 0.0 {
								// 左移動: sel の左端が blocker と重なりそうなら後ろでスナップ
								if sel_start + constrained_delta < b_end && sel_end + constrained_delta > b_start {
									constrained_delta = constrained_delta.max((b_end - sel_start).min(0.0));
								}
							}
						}
					}

					// 先頭クリップが 0 より前に出ないよう制約
					let min_start: f32 = selected
						.iter()
						.filter_map(|&(t, id)| {
							model
								.tracks
								.get(t)
								.and_then(|tr| tr.clips.iter().find(|c| c.id == id))
								.map(|c| c.start_time)
						})
						.fold(f32::INFINITY, f32::min);
					if constrained_delta < 0.0 {
						constrained_delta = constrained_delta.max(-min_start);
					}

					if constrained_delta != 0.0 {
						for (sel_track, sel_clip) in selected {
							if let Some(track) = model.tracks.get_mut(sel_track) {
								if let Some(clip) = track.clips.iter_mut().find(|c| c.id == sel_clip) {
									clip.start_time += constrained_delta;
								}
							}
						}
					}
					return (true, None);
				}

				let cursor_time = self.x_to_time(pos.x, timeline_left);
				let proposed = (cursor_time + offset).max(0.0);

				// Get clip duration
				let clip_duration = model
					.tracks
					.get(track_id)
					.and_then(|t| t.clips.iter().find(|c| c.id == clip_id))
					.map(|c| c.duration)
					.unwrap_or(1.0);

				// Get current start to determine drag direction
				let current_start = model
					.tracks
					.get(track_id)
					.and_then(|t| t.clips.iter().find(|c| c.id == clip_id))
					.map(|c| c.start_time)
					.unwrap_or(0.0);

				let drag_direction = proposed - current_start;

				if self.alt_pressed {
					// まず前フレームで縮めたクリップを元のサイズに戻す
					if let Some((orig_track, orig_id, orig_start, orig_dur)) = self.alt_shrinking_original.take() {
						if let Some(track) = model.tracks.get_mut(orig_track) {
							if let Some(c) = track.clips.iter_mut().find(|c| c.id == orig_id) {
								c.start_time = orig_start;
								c.duration = orig_dur;
							}
						}
					}

					let mut shrunk_clip_id: Option<usize> = None;

					if let Some(track) = model.tracks.get_mut(track_id) {
						let mut clips: Vec<_> = track.clips.iter_mut().filter(|c| c.id != clip_id).collect();
						clips.sort_by(|a, b| {
							a.start_time
								.partial_cmp(&b.start_time)
								.unwrap_or(std::cmp::Ordering::Equal)
						});

						// D の右端が C の中にある → C の左端を押し出す (右移動で突入した状態を左戻しでも維持)
						for c in clips.iter_mut() {
							if c.start_time < proposed + clip_duration
								&& c.start_time + c.duration > proposed + clip_duration
							{
								self.alt_shrinking_original = Some((track_id, c.id, c.start_time, c.duration));
								shrunk_clip_id = Some(c.id);
								let old_end = c.start_time + c.duration;
								c.start_time = proposed + clip_duration;
								c.duration = (old_end - c.start_time).max(MIN_CLIP_DURATION);
								break;
							}
						}

						// D の左端が C の中にある → C の右端を切る (右端衝突がない場合のみ)
						if shrunk_clip_id.is_none() {
							for c in clips.iter_mut().rev() {
								if c.start_time < proposed && c.start_time + c.duration > proposed {
									self.alt_shrinking_original = Some((track_id, c.id, c.start_time, c.duration));
									shrunk_clip_id = Some(c.id);
									c.duration = (proposed - c.start_time).max(MIN_CLIP_DURATION);
									break;
								}
							}
						}
					}

					self.alt_shrinking_clip = shrunk_clip_id.map(|cid| (track_id, cid));

					// Move the dragged clip normally
					if let Some(track) = model.tracks.get_mut(track_id)
						&& let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id)
					{
						clip.start_time = proposed;
					}
				} else {
					// Alt を離した場合は縮めていたクリップを復元する
					if let Some((orig_track, orig_id, orig_start, orig_dur)) = self.alt_shrinking_original.take() {
						if let Some(track) = model.tracks.get_mut(orig_track) {
							if let Some(c) = track.clips.iter_mut().find(|c| c.id == orig_id) {
								c.start_time = orig_start;
								c.duration = orig_dur;
							}
						}
					}
					self.alt_shrinking_clip = None;
					// Normal mode: snap to avoid overlap, don't place if no room
					let mut constrained_time = proposed;
					let mut could_place = true;

					if let Some(track) = model.tracks.get(track_id) {
						// Sort clips by start time to process in order
						let mut clips: Vec<_> = track.clips.iter().filter(|c| c.id != clip_id).collect();
						clips.sort_by(|a, b| {
							a.start_time
								.partial_cmp(&b.start_time)
								.unwrap_or(std::cmp::Ordering::Equal)
						});

						if drag_direction > 0.0 {
							// Moving right: find first clip that overlaps and would block
							for c in clips.iter() {
								if proposed < c.start_time + c.duration && proposed + clip_duration > c.start_time {
									// Cursor crossed past the blocker's right edge → snap after it
									let target_pos = if cursor_time > c.start_time + c.duration {
										c.start_time + c.duration
									} else {
										// Normal: snap before blocker
										c.start_time - clip_duration
									};
									let has_room = target_pos >= 0.0
										&& clips.iter().all(|other| {
											other.id == c.id
												|| target_pos >= other.start_time + other.duration
												|| target_pos + clip_duration <= other.start_time
										});
									if has_room {
										constrained_time = target_pos;
									} else {
										could_place = false;
									}
									break;
								}
							}
						} else if drag_direction < 0.0 {
							// Moving left: find first clip that overlaps
							for c in clips.iter() {
								if proposed < c.start_time + c.duration && proposed + clip_duration > c.start_time {
									// Cursor crossed past the blocker's left edge → snap before it
									let target_pos = if cursor_time < c.start_time {
										c.start_time - clip_duration
									} else {
										// Normal: snap after blocker
										c.start_time + c.duration
									};
									let has_room = target_pos >= 0.0
										&& clips.iter().all(|other| {
											other.id == c.id
												|| target_pos >= other.start_time + other.duration
												|| target_pos + clip_duration <= other.start_time
										});
									if has_room {
										constrained_time = target_pos;
									} else {
										could_place = false;
									}
									break;
								}
							}
						}
					}

					// Apply the constrained position only if valid
					if could_place
						&& let Some(track) = model.tracks.get_mut(track_id)
						&& let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id)
					{
						clip.start_time = constrained_time.max(0.0);
					}
				}
				(true, None)
			}
			DragState::ClipResizeLeft {
				track_id,
				clip_id,
				original_start,
				original_duration,
			} => {
				let time = self.x_to_time(pos.x, timeline_left).max(0.0);
				let fixed_end = original_start + original_duration;

				// 左にあるクリップの右端のうち最も大きいもの（左端の下限）
				let left_bound = model
					.tracks
					.get(track_id)
					.map(|t| {
						t.clips
							.iter()
							.filter(|c| c.id != clip_id && c.start_time + c.duration <= fixed_end)
							.map(|c| c.start_time + c.duration)
							.fold(0.0_f32, f32::max)
					})
					.unwrap_or(0.0);

				if let Some(clip) = Self::find_clip_mut(model, track_id, clip_id) {
					let delta = time - original_start;
					let new_duration = (original_duration - delta).max(MIN_CLIP_DURATION);
					let new_start = (fixed_end - new_duration)
						.max(left_bound)
						.min(fixed_end - MIN_CLIP_DURATION);
					clip.start_time = new_start;
					clip.duration = fixed_end - new_start;
				}
				(true, None)
			}
			DragState::ClipResizeRight { track_id, clip_id } => {
				let end_time = self.x_to_time(pos.x, timeline_left);

				// 右にあるクリップの左端のうち最も小さいもの（右端の上限）
				let clip_start = model
					.tracks
					.get(track_id)
					.and_then(|t| t.clips.iter().find(|c| c.id == clip_id))
					.map(|c| c.start_time)
					.unwrap_or(0.0);
				let right_bound = model
					.tracks
					.get(track_id)
					.map(|t| {
						t.clips
							.iter()
							.filter(|c| c.id != clip_id && c.start_time >= clip_start)
							.map(|c| c.start_time)
							.fold(f32::INFINITY, f32::min)
					})
					.unwrap_or(f32::INFINITY);

				if let Some(clip) = Self::find_clip_mut(model, track_id, clip_id) {
					let clamped_end = end_time.min(right_bound);
					clip.duration = (clamped_end - clip.start_time).max(MIN_CLIP_DURATION);
				}
				(true, None)
			}
			DragState::ClipDuplicating {
				track_id,
				clip_id: _,
				original_clip,
				offset,
			} => {
				// During duplicate drag, we add a duplicate clip to model at proposed position
				// The original clip stays where it is, and we drag the duplicate
				let proposed = self.x_to_time(pos.x, timeline_left) + offset;
				let proposed = proposed.max(0.0);

				if let Some(track) = model.tracks.get_mut(track_id) {
					// Check if we already have a pending duplicate clip
					let duplicate_exists = track.clips.iter().any(|c| c.id == original_clip.id + 10000);

					if !duplicate_exists {
						// Create a new clip as duplicate (use id + 10000 to mark as duplicate)
						let mut duplicate = original_clip.clone();
						duplicate.id = original_clip.id + 10000;
						duplicate.start_time = proposed;
						track.clips.push(duplicate);
					} else {
						// Update position of existing duplicate
						if let Some(clip) = track.clips.iter_mut().find(|c| c.id == original_clip.id + 10000) {
							clip.start_time = proposed;
						}
					}
				}
				(true, None)
			}
			DragState::ReorderTrack {
				from_index: _,
				current_index: _,
			} => {
				// Calculate gap index based on mouse Y position
				// Gap 0 = before track 0, Gap 1 = between track 0 and 1, etc.
				let track_labels_top = bounds.y + RANGE_SLIDER_HEIGHT + RULER_HEIGHT;
				let mut relative_y = pos.y - track_labels_top - self.scroll_offset.y;

				// Adjust for preview gap if tracks are shifted
				if let Some((from_idx, to_idx)) = self.get_reorder_indices() {
					let insert_idx = if to_idx < from_idx { to_idx } else { to_idx + 1 };
					let gap_position = (relative_y / TRACK_HEIGHT).floor() as usize;
					if gap_position > insert_idx {
						relative_y += TRACK_HEIGHT; // Compensate for shifted tracks
					}
				}

				let gap_index = ((relative_y + TRACK_HEIGHT * 0.5) / TRACK_HEIGHT).floor() as isize;
				let num_gaps = model.tracks.len() as isize + 1;

				let clamped_gap = gap_index.clamp(0, num_gaps - 1);

				if let DragState::ReorderTrack {
					from_index: _,
					current_index,
				} = &mut self.drag_state
				{
					*current_index = clamped_gap as usize;
				}
				(true, None)
			}
			DragState::RangeSelect { start } => {
				let x1 = start.x.min(pos.x);
				let x2 = start.x.max(pos.x);
				let y1 = start.y.min(pos.y);
				let y2 = start.y.max(pos.y);
				let rect = Rectangle {
					x: x1,
					y: y1,
					width: x2 - x1,
					height: y2 - y1,
				};
				self.selection_rect = Some(rect);
				self.selected_clips = self.find_clips_in_rect(model, rect, bounds);
				(false, None)
			}
			DragState::RangeSliderLeft {
				start_x,
				init_visible_start,
				fixed_visible_end,
				total_duration,
				slider_width,
			} => {
				let dx = pos.x - start_x;
				let new_start = (init_visible_start + dx / slider_width * total_duration).max(0.0);
				let new_start = new_start.min(fixed_visible_end - 0.5);
				let visible_duration = (fixed_visible_end - new_start).max(0.5);
				let content_w = (self.viewport_width - TRACK_LABEL_WIDTH).max(1.0);
				self.time_scale = (content_w / (visible_duration * PIXELS_PER_SECOND)).clamp(MIN_SCALE, MAX_SCALE);
				self.scroll_offset.x = (-new_start * PIXELS_PER_SECOND * self.time_scale).min(0.0);
				(true, None)
			}
			DragState::RangeSliderRight {
				start_x,
				fixed_visible_start,
				init_visible_end,
				total_duration,
				slider_width,
			} => {
				let dx = pos.x - start_x;
				let new_end = (init_visible_end + dx / slider_width * total_duration).max(fixed_visible_start + 0.5);
				let visible_duration = (new_end - fixed_visible_start).max(0.5);
				let content_w = (self.viewport_width - TRACK_LABEL_WIDTH).max(1.0);
				self.time_scale = (content_w / (visible_duration * PIXELS_PER_SECOND)).clamp(MIN_SCALE, MAX_SCALE);
				self.scroll_offset.x = (-fixed_visible_start * PIXELS_PER_SECOND * self.time_scale).min(0.0);
				(true, None)
			}
			DragState::RangeSliderMiddle {
				start_x,
				init_visible_start,
				total_duration,
				slider_width,
			} => {
				let dx = pos.x - start_x;
				let new_start = (init_visible_start + dx / slider_width * total_duration).max(0.0);
				self.scroll_offset.x = (-new_start * PIXELS_PER_SECOND * self.time_scale).min(0.0);
				(true, None)
			}
			DragState::None => {
				self.hovered_clip = self.find_clip_at(model, pos, bounds);
				(false, None)
			}
		}
	}

	/// マウスリリース時の処理
	///
	/// プレイヘッドドラッグ終了時は最終時間を返す
	pub(crate) fn handle_mouse_release(&mut self, _model: &mut TimelineModel) -> (bool, Option<f32>) {
		if let DragState::RangeSelect { .. } = self.drag_state {
			self.selection_rect = None;
			self.drag_state = DragState::None;
			return (false, None);
		}

		if let DragState::ReorderTrack {
			from_index,
			current_index,
		} = self.drag_state
		{
			if from_index != current_index {
				self.pending_reorder = Some((from_index, current_index));
			}
			// Trigger glow animation on the final position
			self.reorder_glow = Some((current_index, 1.0));
			self.drag_state = DragState::None;
			return (true, None);
		}

		// Handle ClipDuplicating -> finalize the duplicate
		if let DragState::ClipDuplicating {
			track_id: _,
			clip_id: _,
			original_clip: _,
			offset: _,
		} = &self.drag_state
		{
			// The duplicate was already added to model during drag (in handle_mouse_move)
			// Just convert to regular Clip drag state for the final position
			// The duplicate stays, original stays
			self.drag_state = DragState::None;
			return (true, None);
		}

		if !matches!(self.drag_state, DragState::None) {
			self.drag_state = DragState::None;
			self.alt_shrinking_clip = None;
			self.alt_shrinking_original = None; // 復元せず確定
			return (true, None);
		}
		(false, None)
	}

	/// Get the current reorder indices if reordering is in progress
	pub fn get_reorder_indices(&self) -> Option<(usize, usize)> {
		match &self.drag_state {
			DragState::ReorderTrack {
				from_index,
				current_index,
			} if from_index != current_index => Some((*from_index, *current_index)),
			_ => None,
		}
	}

	/// Get pending reorder and clear it
	pub fn take_reorder(&mut self) -> Option<(usize, usize)> {
		self.pending_reorder.take()
	}

	pub(crate) fn handle_scroll(&mut self, delta: mouse::ScrollDelta, current_time: f32) -> bool {
		match delta {
			_ if self.ctrl_pressed => {
				let dy = match delta {
					mouse::ScrollDelta::Lines { y, .. } => y * SCROLL_MULTIPLIER,
					mouse::ScrollDelta::Pixels { y, .. } => y,
				};
				if dy > 0.0 {
					self.zoom_in(current_time);
				} else if dy < 0.0 {
					self.zoom_out(current_time);
				}
			}
			mouse::ScrollDelta::Lines { x, y } => {
				self.apply_scroll_delta(x * SCROLL_MULTIPLIER, y * SCROLL_MULTIPLIER);
			}
			mouse::ScrollDelta::Pixels { x, y } => {
				self.apply_scroll_delta(x, y);
			}
		}
		true
	}

	pub(crate) fn update_modifiers(&mut self, modifiers: keyboard::Modifiers) {
		self.ctrl_pressed = modifiers.command();
		self.alt_pressed = modifiers.alt();
	}

	pub(crate) fn handle_keyboard(&mut self, key: Key, modifiers: keyboard::Modifiers, current_time: f32) -> bool {
		if modifiers.command() {
			match key {
				Key::Character(ref c) if c == "=" || c == "+" => {
					self.zoom_in(current_time);
					return true;
				}
				Key::Character(ref c) if c == "-" => {
					self.zoom_out(current_time);
					return true;
				}
				_ => {}
			}
		}
		false
	}

	pub(crate) fn get_mouse_interaction(
		&self,
		model: &TimelineModel,
		pos: Point,
		bounds: Rectangle,
	) -> mouse::Interaction {
		match &self.drag_state {
			DragState::ClipResizeLeft { .. } | DragState::ClipResizeRight { .. } => {
				return mouse::Interaction::ResizingHorizontally;
			}
			DragState::Clip { .. } => {
				return mouse::Interaction::Grabbing;
			}
			DragState::RangeSelect { .. } => {
				return mouse::Interaction::Crosshair;
			}
			DragState::RangeSliderLeft { .. } | DragState::RangeSliderRight { .. } => {
				return mouse::Interaction::ResizingHorizontally;
			}
			DragState::RangeSliderMiddle { .. } => {
				return mouse::Interaction::Grabbing;
			}
			_ => {}
		}

		let layout = TimelineLayout::from_bounds_absolute(bounds);

		if layout.range_slider.contains(pos) {
			return mouse::Interaction::Grab;
		}

		if layout.content.contains(pos)
			&& let Some((track_id, clip_id)) = self.find_clip_at(model, pos, bounds)
			&& let Some(clip) = Self::find_clip(model, track_id, clip_id)
		{
			let x_start = self.time_to_x(clip.start_time, layout.timeline_left());
			let x_end = self.time_to_x(clip.end_time(), layout.timeline_left());

			if pos.x < x_start + RESIZE_HANDLE_WIDTH || pos.x > x_end - RESIZE_HANDLE_WIDTH {
				return mouse::Interaction::ResizingHorizontally;
			}
			return mouse::Interaction::Grab;
		}

		mouse::Interaction::default()
	}
}
