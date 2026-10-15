//! タイムラインウィジェットの実装
//!
//! After Effects 風のレイヤータイムライン: ラベル色付きのレイヤーバー、
//! キーフレームの菱形、左上のタイムコード、コンポ終端の影を描画する。

use std::collections::HashMap;

use iced::{
	Alignment, Color, Element, Event, Font, Length, Point, Rectangle, Size, Theme,
	alignment::Vertical,
	keyboard::{self, Key},
	mouse,
	widget::{
		Space, button,
		canvas::{self, Canvas, Geometry, LineCap, Path, Stroke, Text},
		container, row,
	},
};

use crate::{
	TimelineClip, TimelineModel,
	icons::{draw_eye_icon, draw_solo_icon},
	interaction::{TimelineInteraction, TimelineLayout},
};
use constants::timeline::{
	colors::{CLIP_HOVERED_OUTLINE, CLIP_RESIZE_HANDLE, CLIP_SELECTED_OUTLINE, CLIP_SHRINKING_OUTLINE},
	*,
};
use core::TrackVisibilityDisplay;

/// Minimum spacing between labelled ruler ticks (px).
const MIN_LABEL_SPACING: f32 = 76.0;
/// Subdivisions between labelled ticks.
const RULER_SUBDIVISIONS: u32 = 5;
const KEYFRAME_RADIUS: f32 = 4.5;

fn rounded_rect(rect: Rectangle, radius: f32) -> Path {
	Path::rounded_rectangle(rect.position(), rect.size(), radius.min(rect.height * 0.5).into())
}

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
	ReorderTrack { from_index: usize, to_index: usize },
	ToggleVisible(usize),
	ToggleSolo(usize),
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
}

//* -------- Public API -------- */
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
		}
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

//* -------- Private API -------- */
impl TimelineWidget<'_> {
	fn zoom_control_view(time_scale: f32) -> Element<'static, TimelineMessage> {
		use constants::icons::{Icon, icon};
		use constants::style::{FONT_TINY, SPACE_1, SPACE_4, TEXT_SECONDARY_COLOR, TOOLBAR_HEIGHT};
		use constants::widgets;

		let icon_button = |glyph: Icon, label: &'static str, message: TimelineMessage| {
			widgets::tip(
				button(widgets::button_body_center(icon(glyph, 13.0, TEXT_SECONDARY_COLOR)))
					.padding(0)
					.width(TOOLBAR_HEIGHT)
					.height(TOOLBAR_HEIGHT)
					.style(widgets::button_ghost)
					.on_press(message),
				label,
				iced::widget::tooltip::Position::Top,
			)
		};

		let zoom_label = widgets::mono_label(format!("{:.0}%", time_scale * 100.0), FONT_TINY + 1.0)
			.color(TEXT_SECONDARY_COLOR)
			.width(40);

		let controls = row![
			Space::new().width(Length::Fill),
			icon_button(Icon::Minus, "Zoom Out  −", TimelineMessage::ZoomOut),
			icon_button(Icon::Plus, "Zoom In  +", TimelineMessage::ZoomIn),
			zoom_label,
			icon_button(Icon::Fit, "Reset Zoom", TimelineMessage::ResetZoom),
		]
		.spacing(SPACE_1 + 2.0)
		.align_y(Alignment::Center);

		container(controls)
			.padding([0.0, SPACE_4])
			.width(Length::Fill)
			.height(TOOLBAR_HEIGHT)
			.center_y(TOOLBAR_HEIGHT)
			.style(widgets::toolbar)
			.into()
	}

	fn draw_timeline(&self, frame: &mut canvas::Frame, bounds: Size) {
		let layout = TimelineLayout::from_bounds(Rectangle::new(Point::ORIGIN, bounds));

		frame.fill_rectangle(Point::ORIGIN, bounds, colors::BACKGROUND);

		self.draw_timeline_content(frame, self.state, self.model, layout.content);
		self.draw_composition_end(frame, layout.ruler, layout.content);
		self.draw_ruler(frame, self.state, layout.ruler, layout.content);
		self.draw_range_slider(frame, self.state, self.model, layout.range_slider);
		self.draw_track_labels(frame, self.state, self.model, layout.track_labels);
		self.draw_corner(
			frame,
			Rectangle::new(Point::ORIGIN, Size::new(TRACK_LABEL_WIDTH, layout.content.y)),
		);
		self.draw_playhead(frame, self.state, layout.ruler, layout.content);
		self.draw_selection_rect(frame);
	}

	/// 左上: 現在時刻のタイムコード（AE のタイムコード表示に相当）
	fn draw_corner(&self, frame: &mut canvas::Frame, rect: Rectangle) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::RULER_BG);
		frame.fill_text(Text {
			content: core::timecode::format(self.current_time, self.fps),
			position: Point::new(rect.x + 10.0, rect.y + rect.height * 0.5),
			color: constants::style::ACCENT_BRIGHT_COLOR,
			size: 14.0.into(),
			font: Font::MONOSPACE,
			align_y: Vertical::Center,
			..Text::default()
		});
		frame.fill_text(Text {
			content: format!("{:.0} fps", self.fps),
			position: Point::new(rect.x + rect.width - 10.0, rect.y + rect.height * 0.5),
			color: colors::TEXT_MUTED,
			size: 9.0.into(),
			align_x: iced::advanced::text::Alignment::Right,
			align_y: Vertical::Center,
			..Text::default()
		});
		self.draw_vertical_line(frame, rect.x + rect.width, rect.y, rect.y + rect.height, colors::BORDER);
		self.draw_horizontal_line(frame, rect.x, rect.x + rect.width, rect.y + rect.height, colors::BORDER);
	}

	fn draw_range_slider(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		model: &TimelineModel,
		rect: Rectangle,
	) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::RANGE_SLIDER_BG);

		let padding = 3.0_f32;
		let track_rect = Rectangle {
			x: rect.x + padding,
			y: rect.y + padding,
			width: (rect.width - padding * 2.0).max(0.0),
			height: rect.height - padding * 2.0,
		};
		frame.fill(&rounded_rect(track_rect, track_rect.height), colors::RANGE_SLIDER_TRACK);

		let total_duration = state.total_timeline_duration(model);
		if total_duration <= 0.0 || track_rect.width <= 0.0 {
			return;
		}

		let norm_start = (state.visible_start_time() / total_duration).clamp(0.0, 1.0);
		let norm_end = (state.visible_end_time() / total_duration).clamp(0.0, 1.0);
		let handle_l = track_rect.x + norm_start * track_rect.width;
		let handle_r = (track_rect.x + norm_end * track_rect.width).max(handle_l + 4.0);

		let handle = rounded_rect(
			Rectangle {
				x: handle_l,
				width: handle_r - handle_l,
				..track_rect
			},
			track_rect.height,
		);
		frame.fill(&handle, colors::RANGE_SLIDER_HANDLE);
		frame.stroke(
			&handle,
			Stroke::default()
				.with_color(colors::RANGE_SLIDER_HANDLE_BORDER)
				.with_width(1.0),
		);

		// Keep both resize grips inside the handle, even at its minimum width.
		let inset = ((handle_r - handle_l) * 0.25).min(4.0);
		let grip_radius = 2.0_f32.min(inset);
		let grip_y = track_rect.y + track_rect.height * 0.5;
		for x in [handle_l + inset, handle_r - inset] {
			frame.fill(
				&Path::circle(Point::new(x, grip_y), grip_radius),
				colors::RANGE_SLIDER_GRIP,
			);
		}
	}

	fn draw_selection_rect(&self, frame: &mut canvas::Frame) {
		let Some(sel_rect) = self.state.selection_rect else {
			return;
		};

		let origin = self.state.canvas_origin;
		let local_rect = Rectangle {
			x: sel_rect.x - origin.x,
			y: sel_rect.y - origin.y,
			width: sel_rect.width,
			height: sel_rect.height,
		};

		let path = rounded_rect(local_rect, 3.0);
		frame.fill(&path, colors::SELECTION_FILL);
		frame.stroke(
			&path,
			Stroke::default().with_color(colors::SELECTION_STROKE).with_width(1.0),
		);
	}

	/// Ruler tick spacing in seconds: the smallest "nice" step whose labels
	/// stay at least [`MIN_LABEL_SPACING`] apart.
	fn label_step(&self) -> f32 {
		let pixels_per_second = PIXELS_PER_SECOND * self.state.time_scale;
		let frame = 1.0 / self.fps.max(1.0);
		[
			frame,
			frame * 2.0,
			frame * 5.0,
			frame * 10.0,
			0.5,
			1.0,
			2.0,
			5.0,
			10.0,
			30.0,
			60.0,
		]
		.into_iter()
		.find(|step| step * pixels_per_second >= MIN_LABEL_SPACING)
		.unwrap_or(120.0)
	}

	fn draw_ruler(&self, frame: &mut canvas::Frame, state: &TimelineInteraction, rect: Rectangle, lanes: Rectangle) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::RULER_BG);
		self.draw_horizontal_line(frame, rect.x, rect.x + rect.width, rect.y + rect.height, colors::BORDER);

		let step = self.label_step();
		let minor = step / RULER_SUBDIVISIONS as f32;
		let visible_start = state.x_to_time(rect.x, rect.x).max(0.0);
		let visible_end = state.x_to_time(rect.x + rect.width, rect.x);
		let first = (visible_start / minor).floor() as i64;
		let last = (visible_end / minor).ceil() as i64;

		for index in first.max(0)..=last {
			let time = index as f32 * minor;
			let x = state.time_to_x(time, rect.x);
			if x < rect.x || x > rect.x + rect.width {
				continue;
			}

			let is_major = index % RULER_SUBDIVISIONS as i64 == 0;
			let (tick_height, tick_color) = if is_major {
				(8.0, colors::TICK_MAJOR)
			} else {
				(4.0, colors::TICK_MINOR)
			};
			self.draw_vertical_line(
				frame,
				x,
				rect.y + rect.height - tick_height,
				rect.y + rect.height,
				tick_color,
			);

			if is_major {
				self.draw_vertical_line(frame, x, lanes.y, lanes.y + lanes.height, colors::GRID_LINE);
				frame.fill_text(Text {
					content: core::timecode::format_ruler(time, self.fps),
					position: Point::new(x + 4.0, rect.y + 4.0),
					color: colors::TEXT_SECONDARY,
					size: 9.5.into(),
					font: Font::MONOSPACE,
					..Text::default()
				});
			}
		}
	}

	/// コンポジション終端より後ろを暗くし、終端に線を引く
	fn draw_composition_end(&self, frame: &mut canvas::Frame, ruler: Rectangle, lanes: Rectangle) {
		let Some(duration) = self.duration else {
			return;
		};
		let x = self.state.time_to_x(duration, lanes.x);
		if x >= lanes.x + lanes.width {
			return;
		}
		let start = x.max(lanes.x);
		frame.fill_rectangle(
			Point::new(start, ruler.y),
			Size::new(lanes.x + lanes.width - start, lanes.y + lanes.height - ruler.y),
			Color::from_rgba(0.0, 0.0, 0.0, 0.32),
		);
		if x >= lanes.x {
			self.draw_vertical_line(
				frame,
				x,
				ruler.y,
				lanes.y + lanes.height,
				Color::from_rgba(1.0, 1.0, 1.0, 0.18),
			);
		}
	}

	fn draw_track_labels(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		model: &TimelineModel,
		rect: Rectangle,
	) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::TRACK_LABEL_BG);

		let reorder_preview = self.reorder_preview;
		let insert_before_idx =
			reorder_preview.map(|(from_idx, to_idx)| if to_idx < from_idx { to_idx } else { to_idx + 1 });

		// with_clip はキャンバス原点がずれていると誤ってクリップされるため使わない。
		// はみ出しは後から描くルーラー／コーナーが覆う。
		{
			for (i, track) in model.tracks.iter().enumerate() {
				let y_offset = match insert_before_idx {
					Some(insert_idx) if i >= insert_idx => TRACK_HEIGHT,
					_ => 0.0,
				};
				let y = rect.y + (i as f32 * TRACK_HEIGHT) + state.scroll_offset.y + y_offset;

				if !state.is_track_visible(y, rect) {
					continue;
				}

				let row_rect = Rectangle::new(Point::new(rect.x, y), Size::new(rect.width, TRACK_HEIGHT));
				let bg_color = if state.selected_track_index == Some(i) {
					Color::from_rgba(1.0, 1.0, 1.0, 0.06)
				} else if i % 2 == 0 {
					colors::TRACK_LABEL_BG
				} else {
					colors::TRACK_LABEL_BG_ALT
				};
				frame.fill_rectangle(row_rect.position(), row_rect.size(), bg_color);

				// ラベル色の帯（掴んで並べ替えるハンドルを兼ねる）
				let rendered = model.is_track_rendered(i);
				let mut label = Self::clip_base_color(i, 0);
				if !rendered {
					label.a = 0.35;
				}
				let strip = Rectangle {
					x: rect.x + (TRACK_HANDLE_WIDTH - 4.0) * 0.5,
					y: y + 6.0,
					width: 4.0,
					height: TRACK_HEIGHT - 12.0,
				};
				frame.fill(&rounded_rect(strip, 2.0), label);

				self.draw_visibility_btn(frame, track_visible_btn_rect(rect.x, y), model.visibility_display(i));
				self.draw_solo_btn(frame, track_solo_btn_rect(rect.x, y), track.solo, track.visible);

				// 1 クリップのトラックは AE のレイヤーのようにクリップ名を表示
				let name = match track.clips.as_slice() {
					[only] => only.name.as_str(),
					_ => track.name.as_str(),
				};
				let name_x = track_name_x(rect.x);
				frame.fill_text(Text {
					content: format!("{}", i + 1),
					position: Point::new(name_x, y + TRACK_HEIGHT * 0.5),
					color: colors::TEXT_MUTED,
					size: 10.0.into(),
					font: Font::MONOSPACE,
					align_y: Vertical::Center,
					..Text::default()
				});
				frame.fill_text(Text {
					content: name.to_string(),
					position: Point::new(name_x + 18.0, y + TRACK_HEIGHT * 0.5),
					color: if rendered {
						colors::TEXT_PRIMARY
					} else {
						colors::TEXT_MUTED
					},
					size: 11.0.into(),
					align_y: Vertical::Center,
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

			// 並べ替え中の挿入位置
			if let Some(insert_idx) = insert_before_idx {
				let gap_y = rect.y + (insert_idx as f32 * TRACK_HEIGHT) + state.scroll_offset.y;
				let gap = rounded_rect(
					Rectangle::new(
						Point::new(rect.x + 2.0, gap_y + 2.0),
						Size::new(rect.width - 4.0, TRACK_HEIGHT - 4.0),
					),
					5.0,
				);
				frame.fill(
					&gap,
					Color {
						a: 0.12,
						..colors::SELECTION
					},
				);
				frame.stroke(
					&gap,
					Stroke::default().with_color(colors::SELECTION_STROKE).with_width(1.5),
				);
			}

			// 並べ替え直後のハイライト
			if let Some((track_idx, alpha)) = self.glowing_track {
				let glow_y = rect.y + (track_idx as f32 * TRACK_HEIGHT) + state.scroll_offset.y;
				frame.fill_rectangle(
					Point::new(rect.x, glow_y),
					Size::new(rect.width, TRACK_HEIGHT),
					Color {
						a: alpha * 0.25,
						..colors::SELECTION
					},
				);
			}
		}

		self.draw_vertical_line(frame, rect.x + rect.width, rect.y, rect.y + rect.height, colors::BORDER);
	}

	fn draw_timeline_content(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		model: &TimelineModel,
		rect: Rectangle,
	) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::TIMELINE_BG);

		let reorder_preview = self.reorder_preview;
		let insert_before_idx =
			reorder_preview.map(|(from_idx, to_idx)| if to_idx < from_idx { to_idx } else { to_idx + 1 });

		{
			for (track_index, track) in model.tracks.iter().enumerate() {
				let y_offset = match insert_before_idx {
					Some(insert_idx) if track_index >= insert_idx => TRACK_HEIGHT,
					_ => 0.0,
				};
				let y = rect.y + (track_index as f32 * TRACK_HEIGHT) + state.scroll_offset.y + y_offset;

				if !state.is_track_visible(y, rect) {
					continue;
				}

				let bg_color = if track_index % 2 == 0 {
					colors::TIMELINE_BG_ALT
				} else {
					colors::TIMELINE_BG_ALT2
				};
				frame.fill_rectangle(Point::new(rect.x, y), Size::new(rect.width, TRACK_HEIGHT), bg_color);

				for clip in &track.clips {
					self.draw_clip(
						frame,
						state,
						rect,
						clip,
						y,
						track_index,
						model.is_track_rendered(track_index),
					);
				}

				self.draw_horizontal_line(
					frame,
					rect.x,
					rect.x + rect.width,
					y + TRACK_HEIGHT,
					colors::BORDER_DARK,
				);
			}

			if let Some(insert_idx) = insert_before_idx {
				let gap_y = rect.y + (insert_idx as f32 * TRACK_HEIGHT) + state.scroll_offset.y;
				frame.fill_rectangle(
					Point::new(rect.x, gap_y),
					Size::new(rect.width, TRACK_HEIGHT),
					Color {
						a: 0.08,
						..colors::SELECTION
					},
				);
			}
		}
	}

	#[allow(clippy::too_many_arguments)]
	fn draw_clip(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		timeline_rect: Rectangle,
		clip: &TimelineClip,
		track_y: f32,
		track_index: usize,
		track_rendered: bool,
	) {
		let x_start = state.time_to_x(clip.start_time, timeline_rect.x);
		let x_end = state.time_to_x(clip.end_time(), timeline_rect.x);

		if x_end < timeline_rect.x || x_start > timeline_rect.x + timeline_rect.width {
			return;
		}

		let clip_rect = Rectangle {
			x: x_start,
			y: track_y + TRACK_PADDING,
			width: (x_end - x_start).max(1.0),
			height: TRACK_HEIGHT - TRACK_PADDING * 2.0,
		};

		let is_selected = state.is_clip_selected(track_index, clip.id);
		let is_hovered = state.is_clip_hovered(track_index, clip.id);
		let is_shrinking = state.alt_shrinking_clip == Some((track_index, clip.id));
		// Duplicate clips (id >= 10000) are shown semi-transparent during drag
		let is_duplicate = clip.id >= 10000;
		let label = Self::clip_base_color(track_index, clip.id);
		let dim = if track_rendered { 1.0 } else { 0.4 };
		let dim = if is_duplicate { dim * 0.5 } else { dim };

		let body = rounded_rect(clip_rect, CLIP_CORNER_RADIUS);
		frame.fill(
			&body,
			Color {
				a: dim,
				..constants::style::mix(label, Color::BLACK, 0.15)
			},
		);

		let outline = if is_selected {
			Stroke::default().with_color(CLIP_SELECTED_OUTLINE).with_width(1.0)
		} else if is_hovered {
			Stroke::default().with_color(CLIP_HOVERED_OUTLINE).with_width(1.0)
		} else {
			Stroke::default()
				.with_color(Color { a: 0.55 * dim, ..label })
				.with_width(1.0)
		};
		frame.stroke(&body, outline);

		if is_shrinking {
			frame.stroke(
				&body,
				Stroke::default().with_color(CLIP_SHRINKING_OUTLINE).with_width(2.0),
			);
		}

		self.draw_keyframes(frame, state, timeline_rect, clip, clip_rect, dim);

		if is_hovered && !is_duplicate {
			self.draw_resize_handles(frame, clip_rect);
		}
	}

	/// クリップ上にキーフレームの菱形を描く
	fn draw_keyframes(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		timeline_rect: Rectangle,
		clip: &TimelineClip,
		clip_rect: Rectangle,
		dim: f32,
	) {
		let Some(times) = self.keyframes.get(&clip.id) else {
			return;
		};
		let center_y = clip_rect.y + clip_rect.height * 0.5;
		for &time in times {
			let outside_clip = time < clip.start_time - f32::EPSILON || time > clip.end_time() + f32::EPSILON;
			let dim = if outside_clip { dim * 0.3 } else { dim };
			let x = state.time_to_x(time, timeline_rect.x);
			let on_playhead = (time - self.current_time).abs() < 0.5 / self.fps.max(1.0);
			let r = if on_playhead {
				KEYFRAME_RADIUS + 1.0
			} else {
				KEYFRAME_RADIUS
			};
			let diamond = Path::new(|builder| {
				builder.move_to(Point::new(x, center_y - r));
				builder.line_to(Point::new(x + r, center_y));
				builder.line_to(Point::new(x, center_y + r));
				builder.line_to(Point::new(x - r, center_y));
				builder.close();
			});
			frame.fill(
				&diamond,
				Color {
					a: dim,
					..colors::KEYFRAME
				},
			);
			frame.stroke(
				&diamond,
				Stroke::default()
					.with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.55 * dim))
					.with_width(1.0),
			);
		}
	}

	fn draw_resize_handles(&self, frame: &mut canvas::Frame, clip_rect: Rectangle) {
		let height = (clip_rect.height - 8.0).max(4.0);
		for x in [clip_rect.x + 4.0, clip_rect.x + clip_rect.width - 6.0] {
			let handle = rounded_rect(
				Rectangle::new(Point::new(x, clip_rect.y + 4.0), Size::new(2.0, height)),
				1.0,
			);
			frame.fill(&handle, CLIP_RESIZE_HANDLE);
		}
	}

	fn draw_playhead(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		ruler_rect: Rectangle,
		timeline_rect: Rectangle,
	) {
		let x = state.time_to_x(self.current_time, timeline_rect.x);

		if x < timeline_rect.x || x > timeline_rect.x + timeline_rect.width {
			return;
		}

		// 線
		let line = Path::line(
			Point::new(x, ruler_rect.y + ruler_rect.height - 4.0),
			Point::new(x, timeline_rect.y + timeline_rect.height),
		);
		frame.stroke(
			&line,
			Stroke::default()
				.with_color(colors::PLAYHEAD)
				.with_width(1.5)
				.with_line_cap(LineCap::Round),
		);

		// ルーラー上のカプセル型ハンドル
		let head = Rectangle {
			x: x - 5.5,
			y: ruler_rect.y + 3.0,
			width: 11.0,
			height: ruler_rect.height - 6.0,
		};
		let head_path = rounded_rect(head, 4.0);
		frame.fill(&head_path, colors::PLAYHEAD);
		frame.stroke(
			&head_path,
			Stroke::default()
				.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.35))
				.with_width(1.0),
		);
		let grip = Path::line(Point::new(x, head.y + 4.0), Point::new(x, head.y + head.height - 4.0));
		frame.stroke(
			&grip,
			Stroke::default()
				.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.85))
				.with_width(1.0)
				.with_line_cap(LineCap::Round),
		);
	}

	// -------------------------------------------------------------------------
	// 描画ヘルパー
	// -------------------------------------------------------------------------

	fn draw_visibility_btn(&self, frame: &mut canvas::Frame, rect: Rectangle, display: TrackVisibilityDisplay) {
		let (icon_color, draw_slash) = match display {
			TrackVisibilityDisplay::Shown => (colors::TEXT_SECONDARY, false),
			TrackVisibilityDisplay::Hidden => (
				Color {
					a: 0.45,
					..colors::TEXT_MUTED
				},
				true,
			),
			// 可視だがソロ除外: アクセント色で第3状態を明示
			TrackVisibilityDisplay::SoloExcluded => (colors::PLAYHEAD, true),
		};

		draw_eye_icon(frame, rect, icon_color);

		if draw_slash {
			let slash = Path::line(
				Point::new(rect.x + 4.0, rect.y + rect.height - 4.0),
				Point::new(rect.x + rect.width - 4.0, rect.y + 4.0),
			);
			frame.stroke(
				&slash,
				Stroke::default()
					.with_color(icon_color)
					.with_width(1.25)
					.with_line_cap(LineCap::Round),
			);
		}
	}

	fn draw_solo_btn(&self, frame: &mut canvas::Frame, rect: Rectangle, active: bool, enabled: bool) {
		let icon_color = if !enabled {
			Color {
				a: 0.3,
				..colors::TEXT_MUTED
			}
		} else if active {
			constants::style::WARNING_COLOR
		} else {
			colors::TEXT_MUTED
		};
		draw_solo_icon(frame, rect, icon_color, active && enabled);
	}

	fn draw_horizontal_line(&self, frame: &mut canvas::Frame, x1: f32, x2: f32, y: f32, color: Color) {
		let line = Path::line(Point::new(x1, y), Point::new(x2, y));
		frame.stroke(&line, Stroke::default().with_color(color).with_width(1.0));
	}

	fn draw_vertical_line(&self, frame: &mut canvas::Frame, x: f32, y1: f32, y2: f32, color: Color) {
		let line = Path::line(Point::new(x, y1), Point::new(x, y2));
		frame.stroke(&line, Stroke::default().with_color(color).with_width(1.0));
	}

	#[inline]
	fn clip_base_color(track_index: usize, _clip_id: usize) -> Color {
		colors::CLIP_PALETTE[track_index % colors::CLIP_PALETTE.len()]
	}
}

impl std::fmt::Debug for TimelineWidget<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("TimelineWidget").finish_non_exhaustive()
	}
}

impl canvas::Program<TimelineMessage> for TimelineWidget<'_> {
	type State = keyboard::Modifiers;

	fn draw(
		&self,
		_canvas_state: &Self::State,
		renderer: &iced::Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		let mut frame = canvas::Frame::new(renderer, bounds.size());
		self.draw_timeline(&mut frame, bounds.size());
		vec![frame.into_geometry()]
	}

	fn update(
		&self,
		canvas_state: &mut Self::State,
		event: &Event,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> Option<canvas::Action<TimelineMessage>> {
		let cursor_pos = cursor
			.position_in(bounds)
			.map(|pos| Point::new(bounds.x + pos.x, bounds.y + pos.y));

		match event {
			Event::Mouse(mouse_event) => match mouse_event {
				mouse::Event::ButtonPressed(mouse::Button::Left) => cursor_pos.map(|position| {
					canvas::Action::publish(TimelineMessage::CanvasEvent(TimelineCanvasEvent::MousePressed {
						position,
						bounds,
						modifiers: *canvas_state,
					}))
				}),
				mouse::Event::ButtonReleased(mouse::Button::Left) => Some(canvas::Action::publish(
					TimelineMessage::CanvasEvent(TimelineCanvasEvent::MouseReleased),
				)),
				mouse::Event::CursorMoved { .. } => cursor_pos.map(|position| {
					canvas::Action::publish(TimelineMessage::CanvasEvent(TimelineCanvasEvent::MouseMoved {
						position,
						bounds,
					}))
				}),
				mouse::Event::WheelScrolled { delta } if cursor_pos.is_some() => Some(canvas::Action::publish(
					TimelineMessage::CanvasEvent(TimelineCanvasEvent::MouseWheelScrolled { delta: *delta, bounds }),
				)),
				_ => None,
			},
			Event::Keyboard(key_event) => match key_event {
				keyboard::Event::ModifiersChanged(modifiers) => {
					*canvas_state = *modifiers;
					None
				}
				keyboard::Event::KeyPressed { key, modifiers, .. } => {
					*canvas_state = *modifiers;
					Some(canvas::Action::publish(TimelineMessage::CanvasEvent(
						TimelineCanvasEvent::KeyPressed {
							key: key.clone(),
							modifiers: *modifiers,
						},
					)))
				}
				keyboard::Event::KeyReleased { key, modifiers, .. } => {
					*canvas_state = *modifiers;
					Some(canvas::Action::publish(TimelineMessage::CanvasEvent(
						TimelineCanvasEvent::KeyReleased {
							key: key.clone(),
							modifiers: *modifiers,
						},
					)))
				}
				_ => None,
			},
			_ => None,
		}
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
					.get_mouse_interaction(self.model, Point::new(bounds.x + pos.x, bounds.y + pos.y), bounds)
			})
			.unwrap_or_default()
	}
}
