//! タイムラインウィジェットの実装

use iced::{
	Color, Element, Event, Length, Point, Rectangle, Size, Theme,
	border::Radius,
	keyboard::{self, Key},
	mouse,
	widget::{
		button,
		canvas::{self, Canvas, Geometry, Path, Stroke, Text},
		column, container, row, slider, text,
	},
};

use crate::{
	TimelineClip, TimelineModel,
	interaction::{TimelineInteraction, TimelineLayout},
	utils::{format_time, lighten_color},
};
use constants::timeline::*;

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
}

/// Canvasから通知されるタイムライン入力イベント
#[derive(Debug, Clone)]
pub enum TimelineCanvasEvent {
	MousePressed {
		position: Point,
		bounds: Rectangle,
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
		modifiers: keyboard::Modifiers,
	},
}

/// タイムラインメッセージ適用結果
#[derive(Debug, Clone, Copy, Default)]
pub struct TimelineUpdate {
	pub playhead_time: Option<f32>,
	pub clip_modified: bool,
}

/// Iced Widget for the timeline pane
/// Timeline displays tracks and clips, and allows user interaction
pub struct TimelineWidget<'a> {
	state: &'a TimelineInteraction,
	model: &'a TimelineModel,
	current_time: f32,
}

// -------- Public API --------
impl<'a> TimelineWidget<'a> {
	/// Create a new TimelineWidget
	pub fn new(state: &'a TimelineInteraction, model: &'a TimelineModel) -> Self {
		Self {
			state,
			model,
			current_time: 0.0,
		}
	}

	/// Create the view element
	pub fn view(self, current_time: f32) -> Element<'a, TimelineMessage> {
		let time_scale = self.state.time_scale;
		let zoom_controls = Self::zoom_control_view(time_scale);
		let timeline_canvas: Element<'a, TimelineMessage> = Canvas::new(Self {
			state: self.state,
			model: self.model,
			current_time,
		})
		.width(Length::Fill)
		.height(Length::Fill)
		.into();

		column![timeline_canvas, zoom_controls].into()
	}
}

// -------- Private API --------
impl TimelineWidget<'_> {
	fn zoom_control_view(time_scale: f32) -> Element<'static, TimelineMessage> {
		let zoom_percent = (time_scale * 100.0) as i32;

		let zoom_out_btn = button(text("-").size(12))
			.on_press(TimelineMessage::ZoomOut)
			.padding([0, 4]);

		let zoom_slider = slider(0.1..=10.0, time_scale, TimelineMessage::ZoomChanged)
			.step(0.1)
			.width(150);

		let zoom_in_btn = button(text("+").size(14))
			.on_press(TimelineMessage::ZoomIn)
			.padding([4, 10]);

		let zoom_label = text(format!("{}%", zoom_percent)).size(12);

		let reset_btn = button(text("Reset").size(12))
			.on_press(TimelineMessage::ResetZoom)
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

	fn draw_timeline(&self, frame: &mut canvas::Frame, bounds: Size) {
		let layout = TimelineLayout::from_bounds(Rectangle::new(Point::ORIGIN, bounds));

		frame.fill_rectangle(Point::ORIGIN, bounds, colors::BACKGROUND);

		self.draw_timeline_content(frame, self.state, self.model, layout.content);
		self.draw_ruler(frame, self.state, layout.ruler);
		self.draw_track_labels(frame, self.state, self.model, layout.track_labels);
		self.draw_playhead(frame, self.state, layout.ruler, layout.content);
	}

	fn draw_ruler(&self, frame: &mut canvas::Frame, state: &TimelineInteraction, rect: Rectangle) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::RULER_BG);

		self.draw_horizontal_line(
			frame,
			rect.x,
			rect.x + rect.width,
			rect.y + rect.height,
			colors::BORDER,
		);

		let visible_start_time = state.x_to_time(rect.x, rect.x).max(0.0);
		let visible_end_time = state.x_to_time(rect.x + rect.width, rect.x);
		let base_interval = state.calculate_tick_interval();

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

	fn draw_track_labels(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		model: &TimelineModel,
		rect: Rectangle,
	) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::TRACK_LABEL_BG);

		self.draw_vertical_line(
			frame,
			rect.x + rect.width,
			rect.y,
			rect.y + rect.height,
			colors::BORDER,
		);

		for (i, track) in model.tracks.iter().enumerate() {
			let y = rect.y + (i as f32 * TRACK_HEIGHT) + state.scroll_offset.y;

			if !state.is_track_visible(y, rect) {
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

	fn draw_timeline_content(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		model: &TimelineModel,
		rect: Rectangle,
	) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::TIMELINE_BG);

		for (track_index, track) in model.tracks.iter().enumerate() {
			let y = rect.y + (track_index as f32 * TRACK_HEIGHT) + state.scroll_offset.y;

			if !state.is_track_visible(y, rect) {
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
				self.draw_clip(frame, state, rect, clip, y, track_index);
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
		state: &TimelineInteraction,
		timeline_rect: Rectangle,
		clip: &TimelineClip,
		track_y: f32,
		track_index: usize,
	) {
		let x_start = state.time_to_x(clip.start_time, timeline_rect.x);
		let x_end = state.time_to_x(clip.end_time(), timeline_rect.x);

		if x_end < timeline_rect.x || x_start > timeline_rect.x + timeline_rect.width {
			return;
		}

		let clip_rect = Rectangle {
			x: x_start,
			y: track_y + TRACK_PADDING,
			width: x_end - x_start,
			height: TRACK_HEIGHT - TRACK_PADDING * 2.0,
		};

		let is_selected = state.is_clip_selected(track_index, clip.id);
		let base_color = Self::clip_base_color(track_index, clip.id);
		let clip_color = if is_selected {
			lighten_color(base_color, 0.2)
		} else {
			base_color
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
		frame.fill_rectangle(
			clip_rect.position(),
			Size::new(RESIZE_HANDLE_VISUAL_WIDTH, clip_rect.height),
			colors::RESIZE_HANDLE,
		);
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
		state: &TimelineInteraction,
		ruler_rect: Rectangle,
		timeline_rect: Rectangle,
	) {
		let x = state.time_to_x(self.current_time, timeline_rect.x);

		if x < timeline_rect.x || x > timeline_rect.x + timeline_rect.width {
			return;
		}

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

	#[inline]
	fn clip_base_color(track_index: usize, clip_id: usize) -> Color {
		const PALETTE: [Color; 8] = [
			Color::from_rgb(0.38, 0.58, 0.95),
			Color::from_rgb(0.24, 0.72, 0.54),
			Color::from_rgb(0.93, 0.61, 0.25),
			Color::from_rgb(0.78, 0.43, 0.90),
			Color::from_rgb(0.28, 0.75, 0.78),
			Color::from_rgb(0.91, 0.42, 0.56),
			Color::from_rgb(0.63, 0.65, 0.29),
			Color::from_rgb(0.55, 0.55, 0.93),
		];

		let index = (clip_id + track_index * 3) % PALETTE.len();
		PALETTE[index]
	}
}

impl std::fmt::Debug for TimelineWidget<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("TimelineWidget").finish_non_exhaustive()
	}
}

impl canvas::Program<TimelineMessage> for TimelineWidget<'_> {
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
		self.draw_timeline(&mut frame, bounds.size());
		vec![frame.into_geometry()]
	}

	fn update(
		&self,
		_canvas_state: &mut Self::State,
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
					canvas::Action::publish(TimelineMessage::CanvasEvent(
						TimelineCanvasEvent::MousePressed { position, bounds },
					))
				}),
				mouse::Event::ButtonReleased(mouse::Button::Left) => Some(canvas::Action::publish(
					TimelineMessage::CanvasEvent(TimelineCanvasEvent::MouseReleased),
				)),
				mouse::Event::CursorMoved { .. } => cursor_pos.map(|position| {
					canvas::Action::publish(TimelineMessage::CanvasEvent(
						TimelineCanvasEvent::MouseMoved { position, bounds },
					))
				}),
				mouse::Event::WheelScrolled { delta } if cursor_pos.is_some() => {
					Some(canvas::Action::publish(TimelineMessage::CanvasEvent(
						TimelineCanvasEvent::MouseWheelScrolled {
							delta: *delta,
							bounds,
						},
					)))
				}
				_ => None,
			},
			Event::Keyboard(key_event) => match key_event {
				keyboard::Event::KeyPressed { key, modifiers, .. } => {
					Some(canvas::Action::publish(TimelineMessage::CanvasEvent(
						TimelineCanvasEvent::KeyPressed {
							key: key.clone(),
							modifiers: *modifiers,
						},
					)))
				}
				keyboard::Event::KeyReleased { modifiers, .. } => Some(canvas::Action::publish(
					TimelineMessage::CanvasEvent(TimelineCanvasEvent::KeyReleased {
						modifiers: *modifiers,
					}),
				)),
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
				self.state.get_mouse_interaction(
					self.model,
					Point::new(bounds.x + pos.x, bounds.y + pos.y),
					bounds,
				)
			})
			.unwrap_or_default()
	}
}
