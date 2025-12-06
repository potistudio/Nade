use iced::{
	Color, Element, Length, Point, Rectangle, Size, Theme, Vector,
	border::Radius,
	mouse,
	widget::canvas::{self, Canvas, Geometry, Path, Stroke, Text},
};

// =============================================================================
// タイムラインの定数
// =============================================================================

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

	pub fn end_time(&self) -> f32 {
		self.start_time + self.duration
	}
}

// =============================================================================
// タイムライントラック
// =============================================================================

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
enum DragState {
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
// メッセージ
// =============================================================================

#[derive(Debug, Clone)]
pub enum Message {
	// 現時点ではcanvasのイベントはprogram内部で処理されるため空
}

// =============================================================================
// タイムラインウィジェット
// =============================================================================

pub struct TimelineWidget;

impl Default for TimelineWidget {
	fn default() -> Self {
		Self
	}
}

impl TimelineWidget {
	pub fn update(&mut self, _message: Message) {
		// 将来の拡張用
	}

	pub fn view(&self) -> Element<'_, Message> {
		Canvas::new(TimelineProgram)
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	}
}

// =============================================================================
// Canvas State - 全ての状態をここで管理
// =============================================================================

#[derive(Debug)]
pub struct TimelineState {
	pub tracks: Vec<TimelineTrack>,
	pub playhead_time: f32,
	pub scroll_offset: Vector,
	pub time_scale: f32,
	pub selected_clip: Option<(usize, usize)>,
	drag_state: DragState,
	next_clip_id: usize,
}

impl Default for TimelineState {
	fn default() -> Self {
		let mut state = Self {
			tracks: Vec::new(),
			playhead_time: 0.0,
			scroll_offset: Vector::new(0.0, 0.0),
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
	fn add_sample_content(&mut self) {
		// トラック1: Video
		let mut track1 = TimelineTrack::new("Video");
		track1.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Intro",
			0.0,
			2.5,
			Color::from_rgb8(66, 133, 244),
		));
		track1.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Main Scene",
			3.0,
			5.0,
			Color::from_rgb8(52, 168, 83),
		));
		track1.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Outro",
			9.0,
			2.0,
			Color::from_rgb8(251, 188, 4),
		));
		self.tracks.push(track1);

		// トラック2: Audio
		let mut track2 = TimelineTrack::new("Audio");
		track2.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"BGM",
			0.0,
			11.0,
			Color::from_rgb8(234, 67, 53),
		));
		self.tracks.push(track2);

		// トラック3: Effects
		let mut track3 = TimelineTrack::new("Effects");
		track3.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Fade In",
			0.0,
			1.0,
			Color::from_rgb8(156, 39, 176),
		));
		track3.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Transition",
			2.5,
			0.5,
			Color::from_rgb8(156, 39, 176),
		));
		track3.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Fade Out",
			10.0,
			1.0,
			Color::from_rgb8(156, 39, 176),
		));
		self.tracks.push(track3);

		// トラック4: Subtitles
		let mut track4 = TimelineTrack::new("Subtitles");
		track4.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Title",
			0.5,
			2.0,
			Color::from_rgb8(0, 188, 212),
		));
		track4.add_clip(TimelineClip::new(
			self.next_clip_id(),
			"Description",
			4.0,
			3.0,
			Color::from_rgb8(0, 188, 212),
		));
		self.tracks.push(track4);
	}

	fn next_clip_id(&mut self) -> usize {
		let id = self.next_clip_id;
		self.next_clip_id += 1;
		id
	}

	fn time_to_x(&self, time: f32, timeline_left: f32) -> f32 {
		timeline_left + (time * PIXELS_PER_SECOND * self.time_scale) + self.scroll_offset.x
	}

	fn x_to_time(&self, x: f32, timeline_left: f32) -> f32 {
		((x - timeline_left) - self.scroll_offset.x) / (PIXELS_PER_SECOND * self.time_scale)
	}

	fn find_clip_at(&self, pos: Point, bounds: Rectangle) -> Option<(usize, usize)> {
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

	/// スクロールオフセットに制限を適用
	fn clamp_scroll_offset(&mut self) {
		// X軸: 左端（時間0）より左にスクロールできないようにする
		if self.scroll_offset.x > 0.0 {
			self.scroll_offset.x = 0.0;
		}

		// Y軸: 上端（最上段トラック）より上にスクロールできないようにする
		if self.scroll_offset.y > 0.0 {
			self.scroll_offset.y = 0.0;
		}
	}
}

// =============================================================================
// Canvas Program
// =============================================================================

struct TimelineProgram;

impl canvas::Program<Message> for TimelineProgram {
	type State = TimelineState;

	fn draw(
		&self,
		state: &Self::State,
		renderer: &iced::Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		// キャッシュを使わず毎フレーム描画（ウィンドウリサイズに対応）
		let mut frame = canvas::Frame::new(renderer, bounds.size());

		// 背景
		frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::from_rgb8(30, 30, 30));

		// レイアウト計算
		let ruler_rect = Rectangle {
			x: TRACK_LABEL_WIDTH,
			y: 0.0,
			width: bounds.width - TRACK_LABEL_WIDTH,
			height: RULER_HEIGHT,
		};

		let track_label_rect = Rectangle {
			x: 0.0,
			y: RULER_HEIGHT,
			width: TRACK_LABEL_WIDTH,
			height: bounds.height - RULER_HEIGHT,
		};

		let timeline_rect = Rectangle {
			x: TRACK_LABEL_WIDTH,
			y: RULER_HEIGHT,
			width: bounds.width - TRACK_LABEL_WIDTH,
			height: bounds.height - RULER_HEIGHT,
		};

		// 描画（順序: タイムラインコンテンツ → ルーラー → トラックラベル → プレイヘッド）
		// トラックラベルを後に描画してクリップとの干渉を防ぐ
		draw_timeline_content(&mut frame, timeline_rect, state);
		draw_ruler(&mut frame, ruler_rect, state);
		draw_track_labels(&mut frame, track_label_rect, state);
		draw_playhead(&mut frame, ruler_rect, timeline_rect, state);

		vec![frame.into_geometry()]
	}

	fn update(
		&self,
		state: &mut Self::State,
		event: canvas::Event,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> (canvas::event::Status, Option<Message>) {
		let timeline_left = bounds.x + TRACK_LABEL_WIDTH;
		let ruler_rect = Rectangle {
			x: bounds.x + TRACK_LABEL_WIDTH,
			y: bounds.y,
			width: bounds.width - TRACK_LABEL_WIDTH,
			height: RULER_HEIGHT,
		};

		let timeline_rect = Rectangle {
			x: bounds.x + TRACK_LABEL_WIDTH,
			y: bounds.y + RULER_HEIGHT,
			width: bounds.width - TRACK_LABEL_WIDTH,
			height: bounds.height - RULER_HEIGHT,
		};

		match event {
			canvas::Event::Mouse(mouse_event) => {
				let cursor_position = cursor.position_in(bounds);

				match mouse_event {
					mouse::Event::ButtonPressed(mouse::Button::Left) => {
						if let Some(pos) = cursor_position {
							let abs_pos = Point::new(bounds.x + pos.x, bounds.y + pos.y);

							// ルーラー上のクリック
							if ruler_rect.contains(abs_pos) {
								let time = state.x_to_time(abs_pos.x, timeline_left).max(0.0);
								state.playhead_time = time;
								state.drag_state = DragState::Playhead;
								return (canvas::event::Status::Captured, None);
							}

							// タイムライン上のクリック
							if timeline_rect.contains(abs_pos) {
								if let Some((track_id, clip_id)) =
									state.find_clip_at(abs_pos, bounds)
								{
									state.selected_clip = Some((track_id, clip_id));

									// リサイズハンドルのチェック
									if let Some(clip) = state.find_clip(track_id, clip_id) {
										let x_start =
											state.time_to_x(clip.start_time, timeline_left);
										let x_end = state.time_to_x(clip.end_time(), timeline_left);
										let handle_width = 8.0;

										if abs_pos.x < x_start + handle_width {
											state.drag_state = DragState::ClipResizeLeft {
												track_id,
												clip_id,
												original_start: clip.start_time,
												original_duration: clip.duration,
											};
										} else if abs_pos.x > x_end - handle_width {
											state.drag_state =
												DragState::ClipResizeRight { track_id, clip_id };
										} else {
											let time = state.x_to_time(abs_pos.x, timeline_left);
											let offset = clip.start_time - time;
											state.drag_state = DragState::Clip {
												track_id,
												clip_id,
												offset,
											};
										}
									}
									return (canvas::event::Status::Captured, None);
								} else {
									// 空白エリアのクリック -> パン開始
									state.selected_clip = None;
									state.drag_state = DragState::Panning {
										start_offset: state.scroll_offset,
										start_cursor: abs_pos,
									};
									return (canvas::event::Status::Captured, None);
								}
							}
						}
					}
					mouse::Event::ButtonReleased(mouse::Button::Left) => {
						if !matches!(state.drag_state, DragState::None) {
							state.drag_state = DragState::None;
							return (canvas::event::Status::Captured, None);
						}
					}
					mouse::Event::CursorMoved { .. } => {
						if let Some(pos) = cursor_position {
							let abs_pos = Point::new(bounds.x + pos.x, bounds.y + pos.y);

							match state.drag_state.clone() {
								DragState::Playhead => {
									state.playhead_time =
										state.x_to_time(abs_pos.x, timeline_left).max(0.0);
									return (canvas::event::Status::Captured, None);
								}
								DragState::Clip {
									track_id,
									clip_id,
									offset,
								} => {
									let time = state.x_to_time(abs_pos.x, timeline_left) + offset;
									if let Some(clip) = state.find_clip_mut(track_id, clip_id) {
										clip.start_time = time.max(0.0);
									}
									return (canvas::event::Status::Captured, None);
								}
								DragState::ClipResizeLeft {
									track_id,
									clip_id,
									original_start,
									original_duration,
								} => {
									let time = state.x_to_time(abs_pos.x, timeline_left).max(0.0);
									if let Some(clip) = state.find_clip_mut(track_id, clip_id) {
										let delta = time - original_start;
										let new_duration = (original_duration - delta).max(0.1);
										clip.start_time =
											original_start + original_duration - new_duration;
										clip.duration = new_duration;
									}
									return (canvas::event::Status::Captured, None);
								}
								DragState::ClipResizeRight { track_id, clip_id } => {
									let end_time = state.x_to_time(abs_pos.x, timeline_left);
									if let Some(clip) = state.find_clip_mut(track_id, clip_id) {
										let new_duration = (end_time - clip.start_time).max(0.1);
										clip.duration = new_duration;
									}
									return (canvas::event::Status::Captured, None);
								}
								DragState::Panning {
									start_offset,
									start_cursor,
								} => {
									let delta = abs_pos - start_cursor;
									state.scroll_offset = start_offset + delta;
									state.clamp_scroll_offset();
									return (canvas::event::Status::Captured, None);
								}
								DragState::None => {}
							}
						}
					}
					mouse::Event::WheelScrolled { delta } => {
						if cursor_position.is_some() {
							match delta {
								mouse::ScrollDelta::Lines { x, y } => {
									state.scroll_offset.x += x * 20.0;
									state.scroll_offset.y += y * 20.0;
									state.clamp_scroll_offset();
									return (canvas::event::Status::Captured, None);
								}
								mouse::ScrollDelta::Pixels { x, y } => {
									state.scroll_offset.x += x;
									state.scroll_offset.y += y;
									state.clamp_scroll_offset();
									return (canvas::event::Status::Captured, None);
								}
							}
						}
					}
					_ => {}
				}
			}
			canvas::Event::Keyboard(keyboard_event) => {
				use iced::keyboard::{Event as KbEvent, Key};
				if let KbEvent::KeyPressed { key, modifiers, .. } = keyboard_event {
					// Cmd/Ctrl + +/- でズーム
					if modifiers.command() {
						match key {
							Key::Character(ref c) if c == "=" || c == "+" => {
								state.time_scale = (state.time_scale * 1.2).min(MAX_SCALE);
								return (canvas::event::Status::Captured, None);
							}
							Key::Character(ref c) if c == "-" => {
								state.time_scale = (state.time_scale / 1.2).max(MIN_SCALE);
								return (canvas::event::Status::Captured, None);
							}
							_ => {}
						}
					}
				}
			}
			_ => {}
		}

		(canvas::event::Status::Ignored, None)
	}

	fn mouse_interaction(
		&self,
		state: &Self::State,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> mouse::Interaction {
		match &state.drag_state {
			DragState::ClipResizeLeft { .. } | DragState::ClipResizeRight { .. } => {
				return mouse::Interaction::ResizingHorizontally;
			}
			DragState::Panning { .. } => {
				return mouse::Interaction::Grabbing;
			}
			DragState::Clip { .. } => {
				return mouse::Interaction::Grabbing;
			}
			_ => {}
		}

		if let Some(pos) = cursor.position_in(bounds) {
			let abs_pos = Point::new(bounds.x + pos.x, bounds.y + pos.y);
			let timeline_left = bounds.x + TRACK_LABEL_WIDTH;
			let timeline_rect = Rectangle {
				x: bounds.x + TRACK_LABEL_WIDTH,
				y: bounds.y + RULER_HEIGHT,
				width: bounds.width - TRACK_LABEL_WIDTH,
				height: bounds.height - RULER_HEIGHT,
			};

			if timeline_rect.contains(abs_pos)
				&& let Some((track_id, clip_id)) = state.find_clip_at(abs_pos, bounds)
				&& let Some(clip) = state.find_clip(track_id, clip_id)
			{
				let x_start = state.time_to_x(clip.start_time, timeline_left);
				let x_end = state.time_to_x(clip.end_time(), timeline_left);
				let handle_width = 8.0;

				if abs_pos.x < x_start + handle_width || abs_pos.x > x_end - handle_width {
					return mouse::Interaction::ResizingHorizontally;
				}
				return mouse::Interaction::Grab;
			}
		}

		mouse::Interaction::default()
	}
}

// =============================================================================
// 描画関数
// =============================================================================

fn draw_ruler(frame: &mut canvas::Frame, rect: Rectangle, state: &TimelineState) {
	// 背景
	frame.fill_rectangle(
		Point::new(rect.x, rect.y),
		Size::new(rect.width, rect.height),
		Color::from_rgb8(45, 45, 45),
	);

	// 下の境界線
	let border = Path::line(
		Point::new(rect.x, rect.y + rect.height),
		Point::new(rect.x + rect.width, rect.y + rect.height),
	);
	frame.stroke(
		&border,
		Stroke::default()
			.with_color(Color::from_rgb8(60, 60, 60))
			.with_width(1.0),
	);

	// 時間目盛り
	let visible_start_time = state.x_to_time(rect.x, rect.x).max(0.0);
	let visible_end_time = state.x_to_time(rect.x + rect.width, rect.x);

	// 目盛り間隔を計算（スケールに応じて調整）
	let base_interval = if state.time_scale < 0.3 {
		5.0
	} else if state.time_scale < 1.0 {
		2.0
	} else if state.time_scale < 3.0 {
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

		let x = state.time_to_x(time, rect.x);
		if x < rect.x || x > rect.x + rect.width {
			continue;
		}

		// 主目盛り
		let is_major = (i % 2) == 0;
		let tick_height = if is_major { 10.0 } else { 6.0 };
		let tick_color = if is_major {
			Color::from_rgb8(150, 150, 150)
		} else {
			Color::from_rgb8(80, 80, 80)
		};

		let tick = Path::line(
			Point::new(x, rect.y + rect.height - tick_height),
			Point::new(x, rect.y + rect.height),
		);
		frame.stroke(
			&tick,
			Stroke::default().with_color(tick_color).with_width(1.0),
		);

		// 時間ラベル（主目盛りのみ）
		if is_major {
			let label = format_time(time);
			let text = Text {
				content: label,
				position: Point::new(x, rect.y + 4.0),
				color: Color::from_rgb8(180, 180, 180),
				size: 10.0.into(),
				..Text::default()
			};
			frame.fill_text(text);
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

fn draw_track_labels(frame: &mut canvas::Frame, rect: Rectangle, state: &TimelineState) {
	// 背景
	frame.fill_rectangle(
		Point::new(rect.x, rect.y),
		Size::new(rect.width, rect.height),
		Color::from_rgb8(40, 40, 40),
	);

	// 右の境界線
	let border = Path::line(
		Point::new(rect.x + rect.width, rect.y),
		Point::new(rect.x + rect.width, rect.y + rect.height),
	);
	frame.stroke(
		&border,
		Stroke::default()
			.with_color(Color::from_rgb8(60, 60, 60))
			.with_width(1.0),
	);

	for (i, track) in state.tracks.iter().enumerate() {
		let y = rect.y + (i as f32 * TRACK_HEIGHT) + state.scroll_offset.y;

		if y + TRACK_HEIGHT < rect.y || y > rect.y + rect.height {
			continue;
		}

		// トラック背景（交互に色を変える）
		let bg_color = if i % 2 == 0 {
			Color::from_rgb8(40, 40, 40)
		} else {
			Color::from_rgb8(35, 35, 35)
		};
		frame.fill_rectangle(
			Point::new(rect.x, y),
			Size::new(rect.width, TRACK_HEIGHT),
			bg_color,
		);

		// トラック名
		let text_color = if track.muted {
			Color::from_rgb8(100, 100, 100)
		} else {
			Color::from_rgb8(220, 220, 220)
		};

		let text = Text {
			content: track.name.clone(),
			position: Point::new(rect.x + 10.0, y + TRACK_HEIGHT / 2.0 - 6.0),
			color: text_color,
			size: 12.0.into(),
			..Text::default()
		};
		frame.fill_text(text);

		// 下の境界線
		let track_border = Path::line(
			Point::new(rect.x, y + TRACK_HEIGHT),
			Point::new(rect.x + rect.width, y + TRACK_HEIGHT),
		);
		frame.stroke(
			&track_border,
			Stroke::default()
				.with_color(Color::from_rgb8(50, 50, 50))
				.with_width(1.0),
		);
	}
}

fn draw_timeline_content(frame: &mut canvas::Frame, rect: Rectangle, state: &TimelineState) {
	// 背景
	frame.fill_rectangle(
		Point::new(rect.x, rect.y),
		Size::new(rect.width, rect.height),
		Color::from_rgb8(25, 25, 25),
	);

	// トラックとクリップを描画
	for (track_index, track) in state.tracks.iter().enumerate() {
		let y = rect.y + (track_index as f32 * TRACK_HEIGHT) + state.scroll_offset.y;

		if y + TRACK_HEIGHT < rect.y || y > rect.y + rect.height {
			continue;
		}

		// トラック背景
		let bg_color = if track_index % 2 == 0 {
			Color::from_rgb8(28, 28, 28)
		} else {
			Color::from_rgb8(32, 32, 32)
		};
		frame.fill_rectangle(
			Point::new(rect.x, y),
			Size::new(rect.width, TRACK_HEIGHT),
			bg_color,
		);

		// クリップを描画
		for clip in &track.clips {
			draw_clip(frame, rect, clip, y, state, track_index);
		}

		// トラック下の境界線
		let track_border = Path::line(
			Point::new(rect.x, y + TRACK_HEIGHT),
			Point::new(rect.x + rect.width, y + TRACK_HEIGHT),
		);
		frame.stroke(
			&track_border,
			Stroke::default()
				.with_color(Color::from_rgb8(40, 40, 40))
				.with_width(1.0),
		);
	}
}

fn draw_clip(
	frame: &mut canvas::Frame,
	timeline_rect: Rectangle,
	clip: &TimelineClip,
	track_y: f32,
	state: &TimelineState,
	track_index: usize,
) {
	let x_start = state.time_to_x(clip.start_time, timeline_rect.x);
	let x_end = state.time_to_x(clip.end_time(), timeline_rect.x);

	// 画面外のクリップはスキップ
	if x_end < timeline_rect.x || x_start > timeline_rect.x + timeline_rect.width {
		return;
	}

	let clip_rect = Rectangle {
		x: x_start,
		y: track_y + TRACK_PADDING,
		width: x_end - x_start,
		height: TRACK_HEIGHT - TRACK_PADDING * 2.0,
	};

	// 選択状態チェック
	let is_selected = state
		.selected_clip
		.map(|(t, c)| t == track_index && c == clip.id)
		.unwrap_or(false);

	// クリップ背景
	let clip_color = if is_selected {
		lighten_color(clip.color, 0.2)
	} else {
		clip.color
	};

	// クリップ（角丸矩形）
	let corner_radius: Radius = 4.0.into();
	let clip_path = Path::rounded_rectangle(
		Point::new(clip_rect.x, clip_rect.y),
		Size::new(clip_rect.width, clip_rect.height),
		corner_radius,
	);
	frame.fill(&clip_path, clip_color);

	// クリップのアウトライン（薄いボーダー）
	let outline = Path::rounded_rectangle(
		Point::new(clip_rect.x, clip_rect.y),
		Size::new(clip_rect.width, clip_rect.height),
		corner_radius,
	);
	frame.stroke(
		&outline,
		Stroke::default()
			.with_color(Color::from_rgba8(255, 255, 255, 0.2))
			.with_width(1.0),
	);

	// 選択時のボーダー
	if is_selected {
		let border = Path::rounded_rectangle(
			Point::new(clip_rect.x, clip_rect.y),
			Size::new(clip_rect.width, clip_rect.height),
			corner_radius,
		);
		frame.stroke(
			&border,
			Stroke::default().with_color(Color::WHITE).with_width(2.0),
		);

		// リサイズハンドル
		let handle_width = 4.0;

		// 左ハンドル
		frame.fill_rectangle(
			Point::new(clip_rect.x, clip_rect.y),
			Size::new(handle_width, clip_rect.height),
			Color::from_rgba8(255, 255, 255, 0.4),
		);

		// 右ハンドル
		frame.fill_rectangle(
			Point::new(clip_rect.x + clip_rect.width - handle_width, clip_rect.y),
			Size::new(handle_width, clip_rect.height),
			Color::from_rgba8(255, 255, 255, 0.4),
		);
	}

	// クリップ名（タイムライン領域内にのみ表示）
	if clip_rect.width > 20.0 {
		// テキストの開始位置をタイムライン領域の左端以上に制限
		let text_x = (clip_rect.x + 6.0).max(timeline_rect.x + 6.0);
		// テキストがクリップの右端を超えないようにする
		if text_x < clip_rect.x + clip_rect.width - 10.0 {
			let text = Text {
				content: clip.name.clone(),
				position: Point::new(text_x, clip_rect.y + clip_rect.height / 2.0 - 6.0),
				color: Color::WHITE,
				size: 11.0.into(),
				..Text::default()
			};
			frame.fill_text(text);
		}
	}
}

fn draw_playhead(
	frame: &mut canvas::Frame,
	ruler_rect: Rectangle,
	timeline_rect: Rectangle,
	state: &TimelineState,
) {
	let x = state.time_to_x(state.playhead_time, timeline_rect.x);

	if x < timeline_rect.x || x > timeline_rect.x + timeline_rect.width {
		return;
	}

	// ルーラー上のヘッド（三角形）
	let head_width = 12.0;
	let head_height = 14.0;

	let head = Path::new(|builder| {
		builder.move_to(Point::new(
			x - head_width / 2.0,
			ruler_rect.y + ruler_rect.height - head_height,
		));
		builder.line_to(Point::new(
			x + head_width / 2.0,
			ruler_rect.y + ruler_rect.height - head_height,
		));
		builder.line_to(Point::new(
			x + head_width / 2.0,
			ruler_rect.y + ruler_rect.height - 4.0,
		));
		builder.line_to(Point::new(x, ruler_rect.y + ruler_rect.height));
		builder.line_to(Point::new(
			x - head_width / 2.0,
			ruler_rect.y + ruler_rect.height - 4.0,
		));
		builder.close();
	});
	frame.fill(&head, Color::from_rgb8(255, 82, 82));

	// タイムライン上のライン
	let line = Path::line(
		Point::new(x, timeline_rect.y),
		Point::new(x, timeline_rect.y + timeline_rect.height),
	);
	frame.stroke(
		&line,
		Stroke::default()
			.with_color(Color::from_rgb8(255, 82, 82))
			.with_width(2.0),
	);
}

fn lighten_color(color: Color, amount: f32) -> Color {
	Color::from_rgb(
		color.r + (1.0 - color.r) * amount,
		color.g + (1.0 - color.g) * amount,
		color.b + (1.0 - color.b) * amount,
	)
}
