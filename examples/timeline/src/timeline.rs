use egui::{
	self, Color32, CornerRadius, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2,
};

// =============================================================================
// タイムラインの定数
// =============================================================================

const HEADER_HEIGHT: f32 = 28.0;
const TRACK_HEIGHT: f32 = 40.0;
const TRACK_PADDING: f32 = 4.0;
const TRACK_LABEL_WIDTH: f32 = 120.0;
const RULER_HEIGHT: f32 = 24.0;
const PIXELS_PER_SECOND: f32 = 100.0;
const MIN_SCALE: f32 = 0.1;
const MAX_SCALE: f32 = 10.0;

// =============================================================================
// タイムラインクリップ
// =============================================================================

#[derive(Clone, Debug)]
pub struct TimelineClip {
	pub id: usize,
	pub name: String,
	pub start_time: f32,
	pub duration: f32,
	pub color: Color32,
}

impl TimelineClip {
	pub fn new(id: usize, name: &str, start_time: f32, duration: f32, color: Color32) -> Self {
		Self {
			id,
			name: name.to_string(),
			start_time,
			duration,
			color,
		}
	}

	pub fn end_time(&self) -> f32 {
		self.start_time + self.duration
	}
}

// =============================================================================
// タイムライントラック
// =============================================================================

#[derive(Clone, Debug)]
pub struct TimelineTrack {
	pub id: usize,
	pub name: String,
	pub clips: Vec<TimelineClip>,
	pub muted: bool,
	pub locked: bool,
}

impl TimelineTrack {
	pub fn new(id: usize, name: &str) -> Self {
		Self {
			id,
			name: name.to_string(),
			clips: Vec::new(),
			muted: false,
			locked: false,
		}
	}

	pub fn add_clip(&mut self, clip: TimelineClip) {
		self.clips.push(clip);
	}
}

// =============================================================================
// ドラッグ状態
// =============================================================================

#[derive(Clone, Debug)]
enum DragState {
	None,
	Playhead(f32),
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
		original_duration: f32,
	},
	Panning(Vec2),
}

// =============================================================================
// タイムラインウィジェット
// =============================================================================

pub struct TimelineWidget {
	pub tracks: Vec<TimelineTrack>,
	pub playhead_time: f32,
	pub scroll_offset: Vec2,
	pub time_scale: f32,
	pub selected_clip: Option<(usize, usize)>,
	drag_state: DragState,
	next_clip_id: usize,
}

impl Default for TimelineWidget {
	fn default() -> Self {
		let mut widget = Self {
			tracks: Vec::new(),
			playhead_time: 0.0,
			scroll_offset: Vec2::ZERO,
			time_scale: 1.0,
			selected_clip: None,
			drag_state: DragState::None,
			next_clip_id: 0,
		};

		// サンプルトラックとクリップを追加
		widget.add_sample_content();
		widget
	}
}

impl TimelineWidget {
	fn add_sample_content(&mut self) {
		// トラック1: Video
		let mut track1 = TimelineTrack::new(0, "Video");
		track1.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Intro",
			0.0,
			2.5,
			Color32::from_rgb(66, 133, 244),
		));
		track1.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Main Scene",
			3.0,
			5.0,
			Color32::from_rgb(52, 168, 83),
		));
		track1.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Outro",
			9.0,
			2.0,
			Color32::from_rgb(251, 188, 4),
		));
		self.tracks.push(track1);

		// トラック2: Audio
		let mut track2 = TimelineTrack::new(1, "Audio");
		track2.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"BGM",
			0.0,
			11.0,
			Color32::from_rgb(234, 67, 53),
		));
		self.tracks.push(track2);

		// トラック3: Effects
		let mut track3 = TimelineTrack::new(2, "Effects");
		track3.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Fade In",
			0.0,
			1.0,
			Color32::from_rgb(156, 39, 176),
		));
		track3.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Transition",
			2.5,
			0.5,
			Color32::from_rgb(156, 39, 176),
		));
		track3.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Fade Out",
			10.0,
			1.0,
			Color32::from_rgb(156, 39, 176),
		));
		self.tracks.push(track3);

		// トラック4: Subtitles
		let mut track4 = TimelineTrack::new(3, "Subtitles");
		track4.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Title",
			0.5,
			2.0,
			Color32::from_rgb(0, 188, 212),
		));
		track4.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Description",
			4.0,
			3.0,
			Color32::from_rgb(0, 188, 212),
		));
		self.tracks.push(track4);
	}

	fn next_clip_id(&mut self) -> usize {
		let id = self.next_clip_id;
		self.next_clip_id += 1;
		id
	}

	pub fn ui(&mut self, ui: &mut Ui) -> Response {
		let available_rect = ui.available_rect_before_wrap();
		let response = ui.allocate_rect(available_rect, Sense::click_and_drag());
		let painter = ui.painter_at(available_rect);

		// 背景
		painter.rect_filled(available_rect, 0.0, Color32::from_rgb(30, 30, 30));

		// レイアウト計算
		let ruler_rect = Rect::from_min_size(
			available_rect.min + Vec2::new(TRACK_LABEL_WIDTH, 0.0),
			Vec2::new(available_rect.width() - TRACK_LABEL_WIDTH, RULER_HEIGHT),
		);

		let track_label_rect = Rect::from_min_size(
			available_rect.min + Vec2::new(0.0, RULER_HEIGHT),
			Vec2::new(TRACK_LABEL_WIDTH, available_rect.height() - RULER_HEIGHT),
		);

		let timeline_rect = Rect::from_min_size(
			available_rect.min + Vec2::new(TRACK_LABEL_WIDTH, RULER_HEIGHT),
			Vec2::new(
				available_rect.width() - TRACK_LABEL_WIDTH,
				available_rect.height() - RULER_HEIGHT,
			),
		);

		// 描画
		self.draw_ruler(&painter, ruler_rect);
		self.draw_track_labels(&painter, track_label_rect);
		self.draw_timeline_content(&painter, timeline_rect);
		self.draw_playhead(&painter, ruler_rect, timeline_rect);

		// インタラクション処理
		self.handle_interaction(ui, &response, ruler_rect, timeline_rect);

		response
	}

	fn time_to_x(&self, time: f32, timeline_rect: Rect) -> f32 {
		timeline_rect.left() + (time * PIXELS_PER_SECOND * self.time_scale) + self.scroll_offset.x
	}

	fn x_to_time(&self, x: f32, timeline_rect: Rect) -> f32 {
		((x - timeline_rect.left()) - self.scroll_offset.x) / (PIXELS_PER_SECOND * self.time_scale)
	}

	fn draw_ruler(&self, painter: &egui::Painter, rect: Rect) {
		// 背景
		painter.rect_filled(rect, 0.0, Color32::from_rgb(45, 45, 45));

		// 下の境界線
		painter.line_segment(
			[rect.left_bottom(), rect.right_bottom()],
			Stroke::new(1.0, Color32::from_rgb(60, 60, 60)),
		);

		// 時間目盛り
		let visible_start_time = self.x_to_time(rect.left(), rect).max(0.0);
		let visible_end_time = self.x_to_time(rect.right(), rect);

		// 目盛り間隔を計算（スケールに応じて調整）
		let base_interval = if self.time_scale < 0.3 {
			5.0
		} else if self.time_scale < 1.0 {
			2.0
		} else if self.time_scale < 3.0 {
			1.0
		} else {
			0.5
		};

		let start_tick = (visible_start_time / base_interval).floor() as i32;
		let end_tick = (visible_end_time / base_interval).ceil() as i32;

		for i in start_tick..=end_tick {
			let time = i as f32 * base_interval;
			if time < 0.0 {
				continue;
			}

			let x = self.time_to_x(time, rect);
			if x < rect.left() || x > rect.right() {
				continue;
			}

			// 主目盛り
			let is_major = (i % 2) == 0;
			let tick_height = if is_major { 10.0 } else { 6.0 };
			let tick_color = if is_major {
				Color32::from_gray(150)
			} else {
				Color32::from_gray(80)
			};

			painter.line_segment(
				[
					Pos2::new(x, rect.bottom() - tick_height),
					Pos2::new(x, rect.bottom()),
				],
				Stroke::new(1.0, tick_color),
			);

			// 時間ラベル（主目盛りのみ）
			if is_major {
				let label = Self::format_time(time);
				painter.text(
					Pos2::new(x, rect.top() + 4.0),
					egui::Align2::CENTER_TOP,
					label,
					egui::FontId::proportional(10.0),
					Color32::from_gray(180),
				);
			}
		}
	}

	fn format_time(seconds: f32) -> String {
		let mins = (seconds / 60.0).floor() as i32;
		let secs = seconds % 60.0;
		if mins > 0 {
			format!("{}:{:05.2}", mins, secs)
		} else {
			format!("{:.2}s", secs)
		}
	}

	fn draw_track_labels(&self, painter: &egui::Painter, rect: Rect) {
		// 背景
		painter.rect_filled(rect, 0.0, Color32::from_rgb(40, 40, 40));

		// 右の境界線
		painter.line_segment(
			[rect.right_top(), rect.right_bottom()],
			Stroke::new(1.0, Color32::from_rgb(60, 60, 60)),
		);

		for (i, track) in self.tracks.iter().enumerate() {
			let y = rect.top() + (i as f32 * TRACK_HEIGHT) + self.scroll_offset.y;

			if y + TRACK_HEIGHT < rect.top() || y > rect.bottom() {
				continue;
			}

			let track_rect = Rect::from_min_size(
				Pos2::new(rect.left(), y),
				Vec2::new(rect.width(), TRACK_HEIGHT),
			);

			// トラック背景（交互に色を変える）
			let bg_color = if i % 2 == 0 {
				Color32::from_rgb(40, 40, 40)
			} else {
				Color32::from_rgb(35, 35, 35)
			};
			painter.rect_filled(track_rect, 0.0, bg_color);

			// トラック名
			let text_color = if track.muted {
				Color32::from_gray(100)
			} else {
				Color32::from_gray(220)
			};

			painter.text(
				Pos2::new(rect.left() + 10.0, y + TRACK_HEIGHT / 2.0),
				egui::Align2::LEFT_CENTER,
				&track.name,
				egui::FontId::proportional(12.0),
				text_color,
			);

			// 下の境界線
			painter.line_segment(
				[
					Pos2::new(rect.left(), y + TRACK_HEIGHT),
					Pos2::new(rect.right(), y + TRACK_HEIGHT),
				],
				Stroke::new(1.0, Color32::from_rgb(50, 50, 50)),
			);
		}
	}

	fn draw_timeline_content(&self, painter: &egui::Painter, rect: Rect) {
		// 背景
		painter.rect_filled(rect, 0.0, Color32::from_rgb(25, 25, 25));

		// トラックとクリップを描画
		for (track_index, track) in self.tracks.iter().enumerate() {
			let y = rect.top() + (track_index as f32 * TRACK_HEIGHT) + self.scroll_offset.y;

			if y + TRACK_HEIGHT < rect.top() || y > rect.bottom() {
				continue;
			}

			// トラック背景
			let track_rect = Rect::from_min_size(
				Pos2::new(rect.left(), y),
				Vec2::new(rect.width(), TRACK_HEIGHT),
			);
			let bg_color = if track_index % 2 == 0 {
				Color32::from_rgb(28, 28, 28)
			} else {
				Color32::from_rgb(32, 32, 32)
			};
			painter.rect_filled(track_rect, 0.0, bg_color);

			// クリップを描画
			for clip in &track.clips {
				self.draw_clip(painter, rect, track_index, clip, y);
			}

			// トラック下の境界線
			painter.line_segment(
				[
					Pos2::new(rect.left(), y + TRACK_HEIGHT),
					Pos2::new(rect.right(), y + TRACK_HEIGHT),
				],
				Stroke::new(1.0, Color32::from_rgb(40, 40, 40)),
			);
		}
	}

	fn draw_clip(
		&self,
		painter: &egui::Painter,
		timeline_rect: Rect,
		track_index: usize,
		clip: &TimelineClip,
		track_y: f32,
	) {
		let x_start = self.time_to_x(clip.start_time, timeline_rect);
		let x_end = self.time_to_x(clip.end_time(), timeline_rect);

		// 画面外のクリップはスキップ
		if x_end < timeline_rect.left() || x_start > timeline_rect.right() {
			return;
		}

		let clip_rect = Rect::from_min_max(
			Pos2::new(x_start, track_y + TRACK_PADDING),
			Pos2::new(x_end, track_y + TRACK_HEIGHT - TRACK_PADDING),
		);

		// 選択状態チェック
		let is_selected = self
			.selected_clip
			.map(|(t, c)| t == track_index && c == clip.id)
			.unwrap_or(false);

		// クリップ背景
		let clip_color = if is_selected {
			lighten_color(clip.color, 0.2)
		} else {
			clip.color
		};

		painter.rect_filled(clip_rect, CornerRadius::same(4), clip_color);

		// 選択時のボーダー
		if is_selected {
			painter.rect_stroke(
				clip_rect,
				CornerRadius::same(4),
				Stroke::new(2.0, Color32::WHITE),
				StrokeKind::Outside,
			);
		}

		// クリップ名
		let text_rect = clip_rect.shrink(4.0);
		if text_rect.width() > 20.0 {
			painter.text(
				text_rect.left_center(),
				egui::Align2::LEFT_CENTER,
				&clip.name,
				egui::FontId::proportional(11.0),
				Color32::WHITE,
			);
		}

		// リサイズハンドル（ホバー時や選択時に表示）
		if is_selected {
			let handle_width = 4.0;

			// 左ハンドル
			let left_handle = Rect::from_min_size(
				clip_rect.left_top(),
				Vec2::new(handle_width, clip_rect.height()),
			);
			painter.rect_filled(
				left_handle,
				CornerRadius::same(2),
				Color32::from_white_alpha(100),
			);

			// 右ハンドル
			let right_handle = Rect::from_min_size(
				Pos2::new(clip_rect.right() - handle_width, clip_rect.top()),
				Vec2::new(handle_width, clip_rect.height()),
			);
			painter.rect_filled(
				right_handle,
				CornerRadius::same(2),
				Color32::from_white_alpha(100),
			);
		}
	}

	fn draw_playhead(&self, painter: &egui::Painter, ruler_rect: Rect, timeline_rect: Rect) {
		let x = self.time_to_x(self.playhead_time, timeline_rect);

		if x < timeline_rect.left() || x > timeline_rect.right() {
			return;
		}

		// ルーラー上のヘッド
		let head_width = 12.0;
		let head_height = 14.0;
		let head_points = vec![
			Pos2::new(x - head_width / 2.0, ruler_rect.bottom() - head_height),
			Pos2::new(x + head_width / 2.0, ruler_rect.bottom() - head_height),
			Pos2::new(x + head_width / 2.0, ruler_rect.bottom() - 4.0),
			Pos2::new(x, ruler_rect.bottom()),
			Pos2::new(x - head_width / 2.0, ruler_rect.bottom() - 4.0),
		];

		painter.add(egui::Shape::convex_polygon(
			head_points,
			Color32::from_rgb(255, 82, 82),
			Stroke::NONE,
		));

		// タイムライン上のライン
		painter.line_segment(
			[
				Pos2::new(x, timeline_rect.top()),
				Pos2::new(x, timeline_rect.bottom()),
			],
			Stroke::new(2.0, Color32::from_rgb(255, 82, 82)),
		);
	}

	fn handle_interaction(
		&mut self,
		ui: &Ui,
		response: &Response,
		ruler_rect: Rect,
		timeline_rect: Rect,
	) {
		let pointer_pos = ui.input(|i| i.pointer.hover_pos());

		// スクロール（ズーム）
		if response.hovered() {
			let scroll_delta = ui.input(|i| i.raw_scroll_delta);

			// Shift + スクロールでズーム
			if ui.input(|i| i.modifiers.shift) && scroll_delta.y != 0.0 {
				let zoom_factor = 1.0 + scroll_delta.y * 0.01;
				self.time_scale = (self.time_scale * zoom_factor).clamp(MIN_SCALE, MAX_SCALE);
			} else {
				// 通常スクロールでパン
				self.scroll_offset.x += scroll_delta.x;
				self.scroll_offset.y += scroll_delta.y;
			}
		}

		// ドラッグ処理
		if response.drag_started()
			&& let Some(pos) = pointer_pos
		{
			self.drag_state = self.determine_drag_target(pos, ruler_rect, timeline_rect);
		}

		if response.dragged()
			&& let Some(pos) = pointer_pos
		{
			self.handle_drag(pos, timeline_rect);
		}

		if response.drag_stopped() {
			self.drag_state = DragState::None;
		}

		// クリックでクリップ選択
		if response.clicked()
			&& let Some(pos) = pointer_pos
		{
			if timeline_rect.contains(pos) {
				self.selected_clip = self.find_clip_at(pos, timeline_rect);
			} else if ruler_rect.contains(pos) {
				// ルーラークリックでプレイヘッド移動
				self.playhead_time = self.x_to_time(pos.x, timeline_rect).max(0.0);
			}
		}
	}

	fn determine_drag_target(
		&mut self,
		pos: Pos2,
		ruler_rect: Rect,
		timeline_rect: Rect,
	) -> DragState {
		// ルーラー上のドラッグ -> プレイヘッド
		if ruler_rect.contains(pos) {
			let time = self.x_to_time(pos.x, timeline_rect).max(0.0);
			self.playhead_time = time;
			return DragState::Playhead(time);
		}

		// タイムライン上のドラッグ
		if timeline_rect.contains(pos) {
			// クリップの検索
			if let Some((track_id, clip_id)) = self.find_clip_at(pos, timeline_rect) {
				self.selected_clip = Some((track_id, clip_id));

				// リサイズハンドルのチェック
				if let Some(clip) = self.find_clip(track_id, clip_id) {
					let x_start = self.time_to_x(clip.start_time, timeline_rect);
					let x_end = self.time_to_x(clip.end_time(), timeline_rect);
					let handle_width = 8.0;

					if pos.x < x_start + handle_width {
						return DragState::ClipResizeLeft {
							track_id,
							clip_id,
							original_start: clip.start_time,
							original_duration: clip.duration,
						};
					} else if pos.x > x_end - handle_width {
						return DragState::ClipResizeRight {
							track_id,
							clip_id,
							original_duration: clip.duration,
						};
					}
				}

				let time = self.x_to_time(pos.x, timeline_rect);
				if let Some(clip) = self.find_clip(track_id, clip_id) {
					let offset = clip.start_time - time;
					return DragState::Clip {
						track_id,
						clip_id,
						offset,
					};
				}
			}

			// 空白エリアのドラッグ -> パン
			return DragState::Panning(self.scroll_offset);
		}

		DragState::None
	}

	fn handle_drag(&mut self, pos: Pos2, timeline_rect: Rect) {
		match self.drag_state.clone() {
			DragState::Playhead(_) => {
				self.playhead_time = self.x_to_time(pos.x, timeline_rect).max(0.0);
			}
			DragState::Clip {
				track_id,
				clip_id,
				offset,
			} => {
				let time = self.x_to_time(pos.x, timeline_rect) + offset;
				if let Some(clip) = self.find_clip_mut(track_id, clip_id) {
					clip.start_time = time.max(0.0);
				}
			}
			DragState::ClipResizeLeft {
				track_id,
				clip_id,
				original_start,
				original_duration,
			} => {
				let time = self.x_to_time(pos.x, timeline_rect).max(0.0);
				if let Some(clip) = self.find_clip_mut(track_id, clip_id) {
					let delta = time - original_start;
					let new_duration = (original_duration - delta).max(0.1);
					clip.start_time = original_start + original_duration - new_duration;
					clip.duration = new_duration;
				}
			}
			DragState::ClipResizeRight {
				track_id,
				clip_id,
				original_duration: _,
			} => {
				let end_time = self.x_to_time(pos.x, timeline_rect);
				if let Some(clip) = self.find_clip_mut(track_id, clip_id) {
					let new_duration = (end_time - clip.start_time).max(0.1);
					clip.duration = new_duration;
				}
			}
			DragState::Panning(_start_offset) => {
				// このケースではdrag_deltaを使う
			}
			DragState::None => {}
		}
	}

	fn find_clip_at(&self, pos: Pos2, timeline_rect: Rect) -> Option<(usize, usize)> {
		for (track_index, track) in self.tracks.iter().enumerate() {
			let track_y =
				timeline_rect.top() + (track_index as f32 * TRACK_HEIGHT) + self.scroll_offset.y;

			if pos.y < track_y + TRACK_PADDING || pos.y > track_y + TRACK_HEIGHT - TRACK_PADDING {
				continue;
			}

			for clip in &track.clips {
				let x_start = self.time_to_x(clip.start_time, timeline_rect);
				let x_end = self.time_to_x(clip.end_time(), timeline_rect);

				if pos.x >= x_start && pos.x <= x_end {
					return Some((track_index, clip.id));
				}
			}
		}
		None
	}

	fn find_clip(&self, track_id: usize, clip_id: usize) -> Option<&TimelineClip> {
		self.tracks
			.get(track_id)
			.and_then(|t| t.clips.iter().find(|c| c.id == clip_id))
	}

	fn find_clip_mut(&mut self, track_id: usize, clip_id: usize) -> Option<&mut TimelineClip> {
		self.tracks
			.get_mut(track_id)
			.and_then(|t| t.clips.iter_mut().find(|c| c.id == clip_id))
	}
}

fn lighten_color(color: Color32, amount: f32) -> Color32 {
	Color32::from_rgb(
		(color.r() as f32 + (255.0 - color.r() as f32) * amount) as u8,
		(color.g() as f32 + (255.0 - color.g() as f32) * amount) as u8,
		(color.b() as f32 + (255.0 - color.b() as f32) * amount) as u8,
	)
}
