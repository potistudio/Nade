use iced::{
	Color, Element, Length, Point, Rectangle, Size, Theme, Vector,
	border::Radius,
	keyboard::{self, Key},
	mouse,
	widget::{
		button,
		canvas::{self, Canvas, Geometry, Path, Stroke, Text},
		column, container, row, slider, text,
	},
};
use std::cell::RefCell;
use std::rc::Rc;

// =============================================================================
// 定数
// =============================================================================

mod consts {
	use iced::Color;

	// レイアウト
	pub const TRACK_HEIGHT: f32 = 40.0;
	pub const TRACK_PADDING: f32 = 4.0;
	pub const TRACK_LABEL_WIDTH: f32 = 120.0;
	pub const RULER_HEIGHT: f32 = 24.0;
	pub const PIXELS_PER_SECOND: f32 = 100.0;

	// スケール制限
	pub const MIN_SCALE: f32 = 0.1;
	pub const MAX_SCALE: f32 = 10.0;

	// インタラクション
	pub const RESIZE_HANDLE_WIDTH: f32 = 8.0;
	pub const RESIZE_HANDLE_VISUAL_WIDTH: f32 = 4.0;
	pub const SCROLL_MULTIPLIER: f32 = 20.0;
	pub const MIN_CLIP_DURATION: f32 = 0.1;
	pub const CLIP_CORNER_RADIUS: f32 = 4.0;

	// 色
	pub mod colors {
		use super::Color;

		pub const BACKGROUND: Color = Color::from_rgb(0.118, 0.118, 0.118);
		pub const RULER_BG: Color = Color::from_rgb(0.176, 0.176, 0.176);
		pub const TRACK_LABEL_BG: Color = Color::from_rgb(0.157, 0.157, 0.157);
		pub const TRACK_LABEL_BG_ALT: Color = Color::from_rgb(0.137, 0.137, 0.137);
		pub const TIMELINE_BG: Color = Color::from_rgb(0.098, 0.098, 0.098);
		pub const TIMELINE_BG_ALT: Color = Color::from_rgb(0.110, 0.110, 0.110);
		pub const TIMELINE_BG_ALT2: Color = Color::from_rgb(0.125, 0.125, 0.125);

		pub const BORDER: Color = Color::from_rgb(0.235, 0.235, 0.235);
		pub const BORDER_SUBTLE: Color = Color::from_rgb(0.196, 0.196, 0.196);
		pub const BORDER_DARK: Color = Color::from_rgb(0.157, 0.157, 0.157);

		pub const TEXT_PRIMARY: Color = Color::from_rgb(0.863, 0.863, 0.863);
		pub const TEXT_SECONDARY: Color = Color::from_rgb(0.706, 0.706, 0.706);
		pub const TEXT_MUTED: Color = Color::from_rgb(0.392, 0.392, 0.392);

		pub const TICK_MAJOR: Color = Color::from_rgb(0.588, 0.588, 0.588);
		pub const TICK_MINOR: Color = Color::from_rgb(0.314, 0.314, 0.314);

		pub const PLAYHEAD: Color = Color::from_rgb(1.0, 0.322, 0.322);

		pub const CLIP_OUTLINE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.2);
		pub const RESIZE_HANDLE: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.4);
	}
}

use consts::*;

// =============================================================================
// レイアウト計算
// =============================================================================

#[derive(Clone, Copy, Debug)]
struct TimelineLayout {
	ruler: Rectangle,
	track_labels: Rectangle,
	content: Rectangle,
}

impl TimelineLayout {
	fn from_bounds(bounds: Rectangle) -> Self {
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

	fn from_bounds_absolute(bounds: Rectangle) -> Self {
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

	fn timeline_left(&self) -> f32 {
		self.content.x
	}
}

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

	#[inline]
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
	ZoomIn,
	ZoomOut,
	ZoomChanged(f32),
	ResetZoom,
	Canvas,
}

// =============================================================================
// タイムラインウィジェット
// =============================================================================

pub struct TimelineWidget {
	state: Rc<RefCell<TimelineState>>,
}

impl Default for TimelineWidget {
	fn default() -> Self {
		Self {
			state: Rc::new(RefCell::new(TimelineState::default())),
		}
	}
}

impl TimelineWidget {
	pub fn update(&mut self, message: Message) {
		let mut state = self.state.borrow_mut();
		match message {
			Message::ZoomIn => state.zoom_in(),
			Message::ZoomOut => state.zoom_out(),
			Message::ZoomChanged(scale) => state.set_time_scale(scale),
			Message::ResetZoom => state.reset_zoom(),
			Message::Canvas => {}
		}
	}

	pub fn view(&self) -> Element<'_, Message> {
		let time_scale = self.state.borrow().time_scale;
		let zoom_controls = self.zoom_controls(time_scale);
		let timeline_canvas: Element<'_, Message> =
			Canvas::new(TimelineProgram::new(Rc::clone(&self.state)))
				.width(Length::Fill)
				.height(Length::Fill)
				.into();

		column![timeline_canvas.map(|_| Message::Canvas), zoom_controls,].into()
	}

	fn zoom_controls(&self, time_scale: f32) -> Element<'_, Message> {
		let zoom_percent = (time_scale * 100.0) as i32;

		let zoom_out_btn = button(text("-").size(14))
			.on_press(Message::ZoomOut)
			.padding([4, 10]);

		let zoom_slider = slider(0.1..=10.0, time_scale, Message::ZoomChanged)
			.step(0.1)
			.width(150);

		let zoom_in_btn = button(text("+").size(14))
			.on_press(Message::ZoomIn)
			.padding([4, 10]);

		let zoom_label = text(format!("{}%", zoom_percent)).size(12);

		let reset_btn = button(text("Reset").size(12))
			.on_press(Message::ResetZoom)
			.padding([4, 8]);

		let controls = row![
			text("Zoom:").size(12),
			zoom_out_btn,
			zoom_slider,
			zoom_in_btn,
			zoom_label,
			reset_btn,
		]
		.spacing(8)
		.align_y(iced::Alignment::Center);

		container(controls)
			.padding(8)
			.width(Length::Fill)
			.style(|_theme| container::Style {
				background: Some(colors::RULER_BG.into()),
				..Default::default()
			})
			.into()
	}
}

// =============================================================================
// タイムライン状態
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
	// -------------------------------------------------------------------------
	// 座標変換
	// -------------------------------------------------------------------------

	#[inline]
	fn time_to_x(&self, time: f32, timeline_left: f32) -> f32 {
		timeline_left + (time * PIXELS_PER_SECOND * self.time_scale) + self.scroll_offset.x
	}

	#[inline]
	fn x_to_time(&self, x: f32, timeline_left: f32) -> f32 {
		((x - timeline_left) - self.scroll_offset.x) / (PIXELS_PER_SECOND * self.time_scale)
	}

	// -------------------------------------------------------------------------
	// クリップ操作
	// -------------------------------------------------------------------------

	fn next_clip_id(&mut self) -> usize {
		let id = self.next_clip_id;
		self.next_clip_id += 1;
		id
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

	fn is_clip_selected(&self, track_index: usize, clip_id: usize) -> bool {
		self.selected_clip == Some((track_index, clip_id))
	}

	// -------------------------------------------------------------------------
	// スクロール
	// -------------------------------------------------------------------------

	fn clamp_scroll_offset(&mut self) {
		self.scroll_offset.x = self.scroll_offset.x.min(0.0);
		self.scroll_offset.y = self.scroll_offset.y.min(0.0);
	}

	fn apply_scroll_delta(&mut self, x: f32, y: f32) {
		self.scroll_offset.x += x;
		self.scroll_offset.y += y;
		self.clamp_scroll_offset();
	}

	// -------------------------------------------------------------------------
	// ズーム
	// -------------------------------------------------------------------------

	fn zoom_in(&mut self) {
		self.time_scale = (self.time_scale * 1.2).min(MAX_SCALE);
	}

	fn zoom_out(&mut self) {
		self.time_scale = (self.time_scale / 1.2).max(MIN_SCALE);
	}

	fn set_time_scale(&mut self, scale: f32) {
		self.time_scale = scale.clamp(MIN_SCALE, MAX_SCALE);
	}

	fn reset_zoom(&mut self) {
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
	// 描画
	// -------------------------------------------------------------------------

	fn draw(&self, frame: &mut canvas::Frame, bounds: Size) {
		let layout = TimelineLayout::from_bounds(Rectangle::new(Point::ORIGIN, bounds));

		frame.fill_rectangle(Point::ORIGIN, bounds, colors::BACKGROUND);

		self.draw_timeline_content(frame, layout.content);
		self.draw_ruler(frame, layout.ruler);
		self.draw_track_labels(frame, layout.track_labels);
		self.draw_playhead(frame, layout.ruler, layout.content);
	}

	fn draw_ruler(&self, frame: &mut canvas::Frame, rect: Rectangle) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::RULER_BG);

		// 下の境界線
		self.draw_horizontal_line(
			frame,
			rect.x,
			rect.x + rect.width,
			rect.y + rect.height,
			colors::BORDER,
		);

		// 時間目盛り
		let visible_start_time = self.x_to_time(rect.x, rect.x).max(0.0);
		let visible_end_time = self.x_to_time(rect.x + rect.width, rect.x);
		let base_interval = self.calculate_tick_interval();

		let start_tick = (visible_start_time / base_interval).floor() as i32;
		let end_tick = (visible_end_time / base_interval).ceil() as i32;

		for i in start_tick..=end_tick {
			let time = i as f32 * base_interval;
			if time < 0.0 {
				continue;
			}

			let x = self.time_to_x(time, rect.x);
			if x < rect.x || x > rect.x + rect.width {
				continue;
			}

			let is_major = (i % 2) == 0;
			let (tick_height, tick_color) = if is_major {
				(10.0, colors::TICK_MAJOR)
			} else {
				(6.0, colors::TICK_MINOR)
			};

			self.draw_vertical_line(
				frame,
				x,
				rect.y + rect.height - tick_height,
				rect.y + rect.height,
				tick_color,
			);

			if is_major {
				frame.fill_text(Text {
					content: format_time(time),
					position: Point::new(x, rect.y + 4.0),
					color: colors::TEXT_SECONDARY,
					size: 10.0.into(),
					..Text::default()
				});
			}
		}
	}

	fn calculate_tick_interval(&self) -> f32 {
		match self.time_scale {
			s if s < 0.3 => 5.0,
			s if s < 1.0 => 2.0,
			s if s < 3.0 => 1.0,
			_ => 0.5,
		}
	}

	fn draw_track_labels(&self, frame: &mut canvas::Frame, rect: Rectangle) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::TRACK_LABEL_BG);

		// 右の境界線
		self.draw_vertical_line(
			frame,
			rect.x + rect.width,
			rect.y,
			rect.y + rect.height,
			colors::BORDER,
		);

		for (i, track) in self.tracks.iter().enumerate() {
			let y = rect.y + (i as f32 * TRACK_HEIGHT) + self.scroll_offset.y;

			if !self.is_track_visible(y, rect) {
				continue;
			}

			let bg_color = if i % 2 == 0 {
				colors::TRACK_LABEL_BG
			} else {
				colors::TRACK_LABEL_BG_ALT
			};
			frame.fill_rectangle(
				Point::new(rect.x, y),
				Size::new(rect.width, TRACK_HEIGHT),
				bg_color,
			);

			let text_color = if track.muted {
				colors::TEXT_MUTED
			} else {
				colors::TEXT_PRIMARY
			};
			frame.fill_text(Text {
				content: track.name.clone(),
				position: Point::new(rect.x + 10.0, y + TRACK_HEIGHT / 2.0 - 6.0),
				color: text_color,
				size: 12.0.into(),
				..Text::default()
			});

			self.draw_horizontal_line(
				frame,
				rect.x,
				rect.x + rect.width,
				y + TRACK_HEIGHT,
				colors::BORDER_SUBTLE,
			);
		}
	}

	fn draw_timeline_content(&self, frame: &mut canvas::Frame, rect: Rectangle) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::TIMELINE_BG);

		for (track_index, track) in self.tracks.iter().enumerate() {
			let y = rect.y + (track_index as f32 * TRACK_HEIGHT) + self.scroll_offset.y;

			if !self.is_track_visible(y, rect) {
				continue;
			}

			let bg_color = if track_index % 2 == 0 {
				colors::TIMELINE_BG_ALT
			} else {
				colors::TIMELINE_BG_ALT2
			};
			frame.fill_rectangle(
				Point::new(rect.x, y),
				Size::new(rect.width, TRACK_HEIGHT),
				bg_color,
			);

			for clip in &track.clips {
				self.draw_clip(frame, rect, clip, y, track_index);
			}

			self.draw_horizontal_line(
				frame,
				rect.x,
				rect.x + rect.width,
				y + TRACK_HEIGHT,
				colors::BORDER_DARK,
			);
		}
	}

	fn draw_clip(
		&self,
		frame: &mut canvas::Frame,
		timeline_rect: Rectangle,
		clip: &TimelineClip,
		track_y: f32,
		track_index: usize,
	) {
		let x_start = self.time_to_x(clip.start_time, timeline_rect.x);
		let x_end = self.time_to_x(clip.end_time(), timeline_rect.x);

		if x_end < timeline_rect.x || x_start > timeline_rect.x + timeline_rect.width {
			return;
		}

		let clip_rect = Rectangle {
			x: x_start,
			y: track_y + TRACK_PADDING,
			width: x_end - x_start,
			height: TRACK_HEIGHT - TRACK_PADDING * 2.0,
		};

		let is_selected = self.is_clip_selected(track_index, clip.id);
		let clip_color = if is_selected {
			lighten_color(clip.color, 0.2)
		} else {
			clip.color
		};

		let corner_radius: Radius = CLIP_CORNER_RADIUS.into();
		let clip_path =
			Path::rounded_rectangle(clip_rect.position(), clip_rect.size(), corner_radius);

		frame.fill(&clip_path, clip_color);
		frame.stroke(
			&clip_path,
			Stroke::default()
				.with_color(colors::CLIP_OUTLINE)
				.with_width(1.0),
		);

		if is_selected {
			frame.stroke(
				&clip_path,
				Stroke::default().with_color(Color::WHITE).with_width(2.0),
			);
			self.draw_resize_handles(frame, clip_rect);
		}

		self.draw_clip_name(frame, clip, clip_rect, timeline_rect);
	}

	fn draw_resize_handles(&self, frame: &mut canvas::Frame, clip_rect: Rectangle) {
		// 左ハンドル
		frame.fill_rectangle(
			clip_rect.position(),
			Size::new(RESIZE_HANDLE_VISUAL_WIDTH, clip_rect.height),
			colors::RESIZE_HANDLE,
		);
		// 右ハンドル
		frame.fill_rectangle(
			Point::new(
				clip_rect.x + clip_rect.width - RESIZE_HANDLE_VISUAL_WIDTH,
				clip_rect.y,
			),
			Size::new(RESIZE_HANDLE_VISUAL_WIDTH, clip_rect.height),
			colors::RESIZE_HANDLE,
		);
	}

	fn draw_clip_name(
		&self,
		frame: &mut canvas::Frame,
		clip: &TimelineClip,
		clip_rect: Rectangle,
		timeline_rect: Rectangle,
	) {
		if clip_rect.width <= 20.0 {
			return;
		}

		let text_x = (clip_rect.x + 6.0).max(timeline_rect.x + 6.0);
		if text_x < clip_rect.x + clip_rect.width - 10.0 {
			frame.fill_text(Text {
				content: clip.name.clone(),
				position: Point::new(text_x, clip_rect.y + clip_rect.height / 2.0 - 6.0),
				color: Color::WHITE,
				size: 11.0.into(),
				..Text::default()
			});
		}
	}

	fn draw_playhead(
		&self,
		frame: &mut canvas::Frame,
		ruler_rect: Rectangle,
		timeline_rect: Rectangle,
	) {
		let x = self.time_to_x(self.playhead_time, timeline_rect.x);

		if x < timeline_rect.x || x > timeline_rect.x + timeline_rect.width {
			return;
		}

		// プレイヘッド（三角形）
		let head_width = 12.0;
		let head_height = 14.0;
		let head_bottom = ruler_rect.y + ruler_rect.height;

		let head = Path::new(|builder| {
			builder.move_to(Point::new(x - head_width / 2.0, head_bottom - head_height));
			builder.line_to(Point::new(x + head_width / 2.0, head_bottom - head_height));
			builder.line_to(Point::new(x + head_width / 2.0, head_bottom - 4.0));
			builder.line_to(Point::new(x, head_bottom));
			builder.line_to(Point::new(x - head_width / 2.0, head_bottom - 4.0));
			builder.close();
		});
		frame.fill(&head, colors::PLAYHEAD);

		// タイムライン上の縦線
		self.draw_vertical_line_with_width(
			frame,
			x,
			timeline_rect.y,
			timeline_rect.y + timeline_rect.height,
			colors::PLAYHEAD,
			2.0,
		);
	}

	// -------------------------------------------------------------------------
	// 描画ヘルパー
	// -------------------------------------------------------------------------

	#[inline]
	fn is_track_visible(&self, track_y: f32, rect: Rectangle) -> bool {
		track_y + TRACK_HEIGHT >= rect.y && track_y <= rect.y + rect.height
	}

	fn draw_horizontal_line(
		&self,
		frame: &mut canvas::Frame,
		x1: f32,
		x2: f32,
		y: f32,
		color: Color,
	) {
		let line = Path::line(Point::new(x1, y), Point::new(x2, y));
		frame.stroke(&line, Stroke::default().with_color(color).with_width(1.0));
	}

	fn draw_vertical_line(
		&self,
		frame: &mut canvas::Frame,
		x: f32,
		y1: f32,
		y2: f32,
		color: Color,
	) {
		let line = Path::line(Point::new(x, y1), Point::new(x, y2));
		frame.stroke(&line, Stroke::default().with_color(color).with_width(1.0));
	}

	fn draw_vertical_line_with_width(
		&self,
		frame: &mut canvas::Frame,
		x: f32,
		y1: f32,
		y2: f32,
		color: Color,
		width: f32,
	) {
		let line = Path::line(Point::new(x, y1), Point::new(x, y2));
		frame.stroke(&line, Stroke::default().with_color(color).with_width(width));
	}

	// -------------------------------------------------------------------------
	// イベントハンドリング
	// -------------------------------------------------------------------------

	fn handle_mouse_press(&mut self, pos: Point, bounds: Rectangle) -> bool {
		let layout = TimelineLayout::from_bounds_absolute(bounds);

		// ルーラー上のクリック
		if layout.ruler.contains(pos) {
			self.playhead_time = self.x_to_time(pos.x, layout.timeline_left()).max(0.0);
			self.drag_state = DragState::Playhead;
			return true;
		}

		// タイムライン上のクリック
		if layout.content.contains(pos) {
			if let Some((track_id, clip_id)) = self.find_clip_at(pos, bounds) {
				self.selected_clip = Some((track_id, clip_id));
				self.start_clip_drag(pos, track_id, clip_id, layout.timeline_left());
				return true;
			}

			// 空白エリア -> パン開始
			self.selected_clip = None;
			self.drag_state = DragState::Panning {
				start_offset: self.scroll_offset,
				start_cursor: pos,
			};
			return true;
		}

		false
	}

	fn start_clip_drag(&mut self, pos: Point, track_id: usize, clip_id: usize, timeline_left: f32) {
		if let Some(clip) = self.find_clip(track_id, clip_id) {
			let x_start = self.time_to_x(clip.start_time, timeline_left);
			let x_end = self.time_to_x(clip.end_time(), timeline_left);

			self.drag_state = if pos.x < x_start + RESIZE_HANDLE_WIDTH {
				DragState::ClipResizeLeft {
					track_id,
					clip_id,
					original_start: clip.start_time,
					original_duration: clip.duration,
				}
			} else if pos.x > x_end - RESIZE_HANDLE_WIDTH {
				DragState::ClipResizeRight { track_id, clip_id }
			} else {
				let time = self.x_to_time(pos.x, timeline_left);
				DragState::Clip {
					track_id,
					clip_id,
					offset: clip.start_time - time,
				}
			};
		}
	}

	fn handle_mouse_move(&mut self, pos: Point, timeline_left: f32) -> bool {
		match self.drag_state.clone() {
			DragState::Playhead => {
				self.playhead_time = self.x_to_time(pos.x, timeline_left).max(0.0);
				true
			}
			DragState::Clip {
				track_id,
				clip_id,
				offset,
			} => {
				let time = self.x_to_time(pos.x, timeline_left) + offset;
				if let Some(clip) = self.find_clip_mut(track_id, clip_id) {
					clip.start_time = time.max(0.0);
				}
				true
			}
			DragState::ClipResizeLeft {
				track_id,
				clip_id,
				original_start,
				original_duration,
			} => {
				let time = self.x_to_time(pos.x, timeline_left).max(0.0);
				if let Some(clip) = self.find_clip_mut(track_id, clip_id) {
					let delta = time - original_start;
					let new_duration = (original_duration - delta).max(MIN_CLIP_DURATION);
					clip.start_time = original_start + original_duration - new_duration;
					clip.duration = new_duration;
				}
				true
			}
			DragState::ClipResizeRight { track_id, clip_id } => {
				let end_time = self.x_to_time(pos.x, timeline_left);
				if let Some(clip) = self.find_clip_mut(track_id, clip_id) {
					clip.duration = (end_time - clip.start_time).max(MIN_CLIP_DURATION);
				}
				true
			}
			DragState::Panning {
				start_offset,
				start_cursor,
			} => {
				let delta = pos - start_cursor;
				self.scroll_offset = start_offset + delta;
				self.clamp_scroll_offset();
				true
			}
			DragState::None => false,
		}
	}

	fn handle_mouse_release(&mut self) -> bool {
		if !matches!(self.drag_state, DragState::None) {
			self.drag_state = DragState::None;
			return true;
		}
		false
	}

	fn handle_scroll(&mut self, delta: mouse::ScrollDelta) -> bool {
		match delta {
			mouse::ScrollDelta::Lines { x, y } => {
				self.apply_scroll_delta(x * SCROLL_MULTIPLIER, y * SCROLL_MULTIPLIER);
			}
			mouse::ScrollDelta::Pixels { x, y } => {
				self.apply_scroll_delta(x, y);
			}
		}
		true
	}

	fn handle_keyboard(&mut self, key: Key, modifiers: keyboard::Modifiers) -> bool {
		if modifiers.command() {
			match key {
				Key::Character(ref c) if c == "=" || c == "+" => {
					self.zoom_in();
					return true;
				}
				Key::Character(ref c) if c == "-" => {
					self.zoom_out();
					return true;
				}
				_ => {}
			}
		}
		false
	}

	fn get_mouse_interaction(&self, pos: Point, bounds: Rectangle) -> mouse::Interaction {
		match &self.drag_state {
			DragState::ClipResizeLeft { .. } | DragState::ClipResizeRight { .. } => {
				return mouse::Interaction::ResizingHorizontally;
			}
			DragState::Panning { .. } | DragState::Clip { .. } => {
				return mouse::Interaction::Grabbing;
			}
			_ => {}
		}

		let layout = TimelineLayout::from_bounds_absolute(bounds);

		if layout.content.contains(pos) {
			if let Some((track_id, clip_id)) = self.find_clip_at(pos, bounds) {
				if let Some(clip) = self.find_clip(track_id, clip_id) {
					let x_start = self.time_to_x(clip.start_time, layout.timeline_left());
					let x_end = self.time_to_x(clip.end_time(), layout.timeline_left());

					if pos.x < x_start + RESIZE_HANDLE_WIDTH || pos.x > x_end - RESIZE_HANDLE_WIDTH
					{
						return mouse::Interaction::ResizingHorizontally;
					}
					return mouse::Interaction::Grab;
				}
			}
		}

		mouse::Interaction::default()
	}
}

// =============================================================================
// Canvas Program
// =============================================================================

struct TimelineProgram {
	state: Rc<RefCell<TimelineState>>,
}

impl TimelineProgram {
	fn new(state: Rc<RefCell<TimelineState>>) -> Self {
		Self { state }
	}
}

impl canvas::Program<Message> for TimelineProgram {
	type State = ();

	fn draw(
		&self,
		_canvas_state: &Self::State,
		renderer: &iced::Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		let mut frame = canvas::Frame::new(renderer, bounds.size());
		self.state.borrow().draw(&mut frame, bounds.size());
		vec![frame.into_geometry()]
	}

	fn update(
		&self,
		_canvas_state: &mut Self::State,
		event: canvas::Event,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> (canvas::event::Status, Option<Message>) {
		let layout = TimelineLayout::from_bounds_absolute(bounds);
		let cursor_position = cursor.position_in(bounds);

		let handled = {
			let mut state = self.state.borrow_mut();
			match event {
				canvas::Event::Mouse(mouse_event) => match mouse_event {
					mouse::Event::ButtonPressed(mouse::Button::Left) => cursor_position
						.map(|pos| {
							state.handle_mouse_press(
								Point::new(bounds.x + pos.x, bounds.y + pos.y),
								bounds,
							)
						})
						.unwrap_or(false),
					mouse::Event::ButtonReleased(mouse::Button::Left) => {
						state.handle_mouse_release()
					}
					mouse::Event::CursorMoved { .. } => cursor_position
						.map(|pos| {
							state.handle_mouse_move(
								Point::new(bounds.x + pos.x, bounds.y + pos.y),
								layout.timeline_left(),
							)
						})
						.unwrap_or(false),
					mouse::Event::WheelScrolled { delta } if cursor_position.is_some() => {
						state.handle_scroll(delta)
					}
					_ => false,
				},
				canvas::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
					state.handle_keyboard(key, modifiers)
				}
				_ => false,
			}
		};

		let status = if handled {
			canvas::event::Status::Captured
		} else {
			canvas::event::Status::Ignored
		};

		(status, None)
	}

	fn mouse_interaction(
		&self,
		_canvas_state: &Self::State,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> mouse::Interaction {
		cursor
			.position_in(bounds)
			.map(|pos| {
				self.state
					.borrow()
					.get_mouse_interaction(Point::new(bounds.x + pos.x, bounds.y + pos.y), bounds)
			})
			.unwrap_or_default()
	}
}

// =============================================================================
// ユーティリティ
// =============================================================================

fn format_time(seconds: f32) -> String {
	let mins = (seconds / 60.0).floor() as i32;
	let secs = seconds % 60.0;
	if mins > 0 {
		format!("{}:{:05.2}", mins, secs)
	} else {
		format!("{:.2}s", secs)
	}
}

fn lighten_color(color: Color, amount: f32) -> Color {
	Color::from_rgb(
		color.r + (1.0 - color.r) * amount,
		color.g + (1.0 - color.g) * amount,
		color.b + (1.0 - color.b) * amount,
	)
}
