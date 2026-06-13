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
use constants::timeline::{
	colors::{CLIP_HOVERED_OUTLINE, CLIP_RESIZE_HANDLE, CLIP_SELECTED_OUTLINE, CLIP_SHRINKING_OUTLINE},
	*,
};

fn squircle_path(center: Point, width: f32, height: f32, corner_radius: f32) -> Path {
	// コーナー半径は短辺の半分を超えられない
	let r = corner_radius.min(width / 2.0).min(height / 2.0);
	let n = 5.0f32; // iOS近似

	let hw = width / 2.0;
	let hh = height / 2.0;

	// 各コーナーの中心
	let corners = [
		Point::new(center.x + hw - r, center.y - hh + r), // 右上
		Point::new(center.x + hw - r, center.y + hh - r), // 右下
		Point::new(center.x - hw + r, center.y + hh - r), // 左下
		Point::new(center.x - hw + r, center.y - hh + r), // 左上
	];

	// コーナーごとの開始角度（ラジアン）
	let start_angles = [
		-std::f32::consts::FRAC_PI_2, // 上→右
		0.0,                          // 右→下
		std::f32::consts::FRAC_PI_2,  // 下→左
		std::f32::consts::PI,         // 左→上
	];

	let corner_steps = 64usize;

	Path::new(|builder| {
		let mut first = true;

		for (corner, &start_angle) in corners.iter().zip(start_angles.iter()) {
			for i in 0..=corner_steps {
				// 0..=corner_steps で π/2 を分割
				let local_t = (i as f32) / (corner_steps as f32); // 0.0..=1.0
				let angle = start_angle + local_t * std::f32::consts::FRAC_PI_2;

				let cos_a = angle.cos();
				let sin_a = angle.sin();

				// 超楕円の曲線を単位円に適用してからrでスケール
				let lx = cos_a.abs().powf(2.0 / n) * cos_a.signum() * r;
				let ly = sin_a.abs().powf(2.0 / n) * sin_a.signum() * r;

				let point = Point::new(corner.x + lx, corner.y + ly);

				if first {
					builder.move_to(point);
					first = false;
				} else {
					builder.line_to(point);
				}
			}
		}

		builder.close();
	})
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
			glowing_track: None,
		}
	}

	/// Create a new TimelineWidget with reorder preview
	pub fn with_reorder_preview(
		state: &'a TimelineInteraction,
		model: &'a TimelineModel,
		current_time: f32,
		reorder_preview: Option<(usize, usize)>,
	) -> Self {
		Self {
			state,
			model,
			current_time,
			reorder_preview,
			glowing_track: state.reorder_glow,
		}
	}

	/// Create the view element
	pub fn view(self, current_time: f32) -> Element<'a, TimelineMessage> {
		Self::with_reorder_preview(self.state, self.model, current_time, None).view_internal()
	}

	pub fn view_internal(self) -> Element<'a, TimelineMessage> {
		let time_scale = self.state.time_scale;
		let zoom_controls = Self::zoom_control_view(time_scale);
		let timeline_canvas: Element<'a, TimelineMessage> = Canvas::new(Self {
			state: self.state,
			model: self.model,
			current_time: self.current_time,
			reorder_preview: self.reorder_preview,
			glowing_track: self.glowing_track,
		})
		.width(Length::Fill)
		.height(Length::Fill)
		.into();

		column![timeline_canvas, zoom_controls].into()
	}
}

//* -------- Private API -------- */
impl TimelineWidget<'_> {
	fn zoom_control_view(time_scale: f32) -> Element<'static, TimelineMessage> {
		let zoom_percent = (time_scale * 100.0) as i32;

		let zoom_out_btn = button(text("−").size(13))
			.on_press(TimelineMessage::ZoomOut)
			.padding([2, 8]);

		let zoom_slider = slider(0.1..=10.0, time_scale, TimelineMessage::ZoomChanged)
			.step(0.1)
			.width(120);

		let zoom_in_btn = button(text("+").size(13))
			.on_press(TimelineMessage::ZoomIn)
			.padding([2, 8]);

		let zoom_label = text(format!("{zoom_percent}%"))
			.size(11)
			.width(40);

		let reset_btn = button(text("1:1").size(11))
			.on_press(TimelineMessage::ResetZoom)
			.padding([2, 6]);

		let controls = row![
			zoom_out_btn,
			zoom_slider,
			zoom_in_btn,
			zoom_label,
			reset_btn,
		]
		.spacing(4)
		.align_y(iced::Alignment::Center);

		container(controls)
			.padding([4, 10])
			.width(Length::Fill)
			.style(|_theme| container::Style {
				background: Some(colors::BACKGROUND.into()),
				border: iced::Border {
					color: colors::BORDER_DARK,
					width: 1.0,
					radius: 0.0.into(),
				},
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
		self.draw_selection_rect(frame);
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

		frame.fill_rectangle(
			local_rect.position(),
			local_rect.size(),
			Color::from_rgba(0.4, 0.7, 1.0, 0.12),
		);

		let sel_path = Path::rectangle(local_rect.position(), local_rect.size());
		frame.stroke(
			&sel_path,
			Stroke::default()
				.with_color(Color::from_rgba(0.4, 0.7, 1.0, 0.75))
				.with_width(1.0),
		);
	}

	fn draw_ruler(&self, frame: &mut canvas::Frame, state: &TimelineInteraction, rect: Rectangle) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::RULER_BG);

		self.draw_horizontal_line(frame, rect.x, rect.x + rect.width, rect.y + rect.height, colors::BORDER);

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
				(5.0, colors::TICK_MINOR)
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

		self.draw_vertical_line(frame, rect.x + rect.width, rect.y, rect.y + rect.height, colors::BORDER);

		let reorder_preview = self.reorder_preview;
		let insert_before_idx =
			reorder_preview.map(|(from_idx, to_idx)| if to_idx < from_idx { to_idx } else { to_idx + 1 });

		for (i, track) in model.tracks.iter().enumerate() {
			// Calculate offset if this track should be shifted due to reordering
			let mut y_offset = 0.0;
			if let Some(insert_idx) = insert_before_idx
				&& i >= insert_idx
			{
				y_offset = TRACK_HEIGHT; // Shift down
			}

			let y = rect.y + (i as f32 * TRACK_HEIGHT) + state.scroll_offset.y + y_offset;

			if !state.is_track_visible(y, rect) {
				continue;
			}

			let bg_color = if i % 2 == 0 {
				colors::TRACK_LABEL_BG
			} else {
				colors::TRACK_LABEL_BG_ALT
			};
			frame.fill_rectangle(Point::new(rect.x, y), Size::new(rect.width, TRACK_HEIGHT), bg_color);

			// Draw track reorder handle on the left side (vertical grip)
			let handle_width = 8.0;
			let handle_rect = Rectangle {
				x: rect.x,
				y,
				width: handle_width,
				height: TRACK_HEIGHT,
			};
			frame.fill_rectangle(handle_rect.position(), handle_rect.size(), colors::TRACK_REORDER_HANDLE);

			// Draw vertical grip lines (3 vertical lines)
			let grip_center_x = rect.x + handle_width / 2.0;
			for line_i in 0..3 {
				let line_x = grip_center_x - 1.5 + (line_i as f32 * 1.5);
				self.draw_vertical_line_with_width(
					frame,
					line_x,
					y + 4.0,
					y + TRACK_HEIGHT - 4.0,
					colors::TEXT_MUTED,
					1.0,
				);
			}

			let text_color = if track.muted {
				colors::TEXT_MUTED
			} else {
				colors::TEXT_PRIMARY
			};
			frame.fill_text(Text {
				content: track.name.clone(),
				position: Point::new(rect.x + 14.0, y + TRACK_HEIGHT / 2.0 - 6.0),
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

		// Draw gap indicator if reordering
		if let Some((from_idx, to_idx)) = reorder_preview {
			let insert_idx = if to_idx < from_idx { to_idx } else { to_idx + 1 };
			let gap_y = rect.y + (insert_idx as f32 * TRACK_HEIGHT) + state.scroll_offset.y;

			// Draw highlight in the gap
			frame.fill_rectangle(
				Point::new(rect.x, gap_y),
				Size::new(rect.width, TRACK_HEIGHT),
				Color::from_rgba(1.0, 1.0, 1.0, 0.15),
			);

			// Draw border
			let gap_path = Path::rectangle(Point::new(rect.x, gap_y), Size::new(rect.width, TRACK_HEIGHT));
			frame.stroke(
				&gap_path,
				Stroke::default().with_color(colors::PLAYHEAD).with_width(2.0),
			);
		}

		// Draw glow effect for track that was just reordered
		if let Some((track_idx, alpha)) = self.glowing_track {
			let glow_y = rect.y + (track_idx as f32 * TRACK_HEIGHT) + state.scroll_offset.y;
			let glow_color = Color::from_rgba(1.0, 1.0, 1.0, alpha * 0.5);

			frame.fill_rectangle(
				Point::new(rect.x, glow_y),
				Size::new(rect.width, TRACK_HEIGHT),
				glow_color,
			);

			let glow_path = Path::rectangle(Point::new(rect.x, glow_y), Size::new(rect.width, TRACK_HEIGHT));
			frame.stroke(
				&glow_path,
				Stroke::default().with_color(colors::PLAYHEAD).with_width(3.0),
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

		let reorder_preview = self.reorder_preview;
		let insert_before_idx =
			reorder_preview.map(|(from_idx, to_idx)| if to_idx < from_idx { to_idx } else { to_idx + 1 });

		for (track_index, track) in model.tracks.iter().enumerate() {
			// Calculate offset if this track should be shifted due to reordering
			let mut y_offset = 0.0;
			if let Some(insert_idx) = insert_before_idx
				&& track_index >= insert_idx
			{
				y_offset = TRACK_HEIGHT; // Shift down
			}

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

		// Draw gap indicator in timeline content area if reordering
		if let Some((_, to_idx)) = reorder_preview {
			let insert_idx = if to_idx < model.tracks.len() {
				if to_idx < reorder_preview.unwrap().0 {
					to_idx
				} else {
					to_idx + 1
				}
			} else {
				model.tracks.len()
			};
			let gap_y = rect.y + (insert_idx as f32 * TRACK_HEIGHT) + state.scroll_offset.y;

			frame.fill_rectangle(
				Point::new(rect.x, gap_y),
				Size::new(rect.width, TRACK_HEIGHT),
				Color::from_rgba(1.0, 1.0, 1.0, 0.15),
			);

			let gap_path = Path::rectangle(Point::new(rect.x, gap_y), Size::new(rect.width, TRACK_HEIGHT));
			frame.stroke(
				&gap_path,
				Stroke::default().with_color(colors::PLAYHEAD).with_width(2.0),
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
		let is_hovered = state.is_clip_hovered(track_index, clip.id);
		let is_shrinking = state.alt_shrinking_clip == Some((track_index, clip.id));
		// Duplicate clips (id >= 10000) are shown semi-transparent during drag
		let is_duplicate = clip.id >= 10000;
		let base_color = Self::clip_base_color(track_index, clip.id);
		let clip_color = if is_selected {
			lighten_color(base_color, 0.2)
		} else {
			base_color
		};

		let clip_path = squircle_path(
			Point::new(
				clip_rect.x + clip_rect.width / 2.0,
				clip_rect.y + clip_rect.height / 2.0,
			),
			clip_rect.width,
			clip_rect.height,
			CLIP_CORNER_RADIUS,
		);

		// Apply transparency for duplicate clips
		if is_duplicate {
			let transparent_color = Color::from_rgba(
				clip_color.r,
				clip_color.g,
				clip_color.b,
				0.5, // 50% opacity
			);
			frame.fill(&clip_path, transparent_color);
			frame.stroke(
				&clip_path,
				Stroke::default().with_color(CLIP_HOVERED_OUTLINE).with_width(1.0),
			);
		} else {
			frame.fill(&clip_path, clip_color);
			frame.stroke(
				&clip_path,
				Stroke::default().with_color(colors::CLIP_OUTLINE).with_width(1.0),
			);

			if is_selected {
				frame.stroke(
					&clip_path,
					Stroke::default().with_color(CLIP_SELECTED_OUTLINE).with_width(1.5),
				);
			}

			if is_shrinking {
				frame.stroke(
					&clip_path,
					Stroke::default().with_color(CLIP_SHRINKING_OUTLINE).with_width(2.0),
				);
			}

			if is_hovered {
				frame.stroke(
					&clip_path,
					Stroke::default().with_color(CLIP_HOVERED_OUTLINE).with_width(1.0),
				);
				self.draw_resize_handles(frame, clip_rect);
			}
		}
	}

	fn draw_resize_handles(&self, frame: &mut canvas::Frame, clip_rect: Rectangle) {
		let handle_thickness = 2.0;
		let handle_height = clip_rect.height - CLIP_CORNER_RADIUS + handle_thickness;
		let handle_margin = CLIP_CORNER_RADIUS / 2.0 - handle_thickness / 2.0;

		// Left handle
		let left_handle_path = Path::rounded_rectangle(
			Point::new(clip_rect.x + handle_margin, clip_rect.y + handle_margin),
			Size::new(handle_thickness, handle_height),
			handle_thickness.into(),
		);
		frame.fill(&left_handle_path, CLIP_RESIZE_HANDLE);

		// Right handle
		let right_handle_path = Path::rounded_rectangle(
			Point::new(
				clip_rect.x + clip_rect.width - handle_margin - handle_thickness,
				clip_rect.y + handle_margin,
			),
			Size::new(handle_thickness, handle_height),
			handle_thickness.into(),
		);
		frame.fill(&right_handle_path, CLIP_RESIZE_HANDLE);
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

	fn draw_horizontal_line(&self, frame: &mut canvas::Frame, x1: f32, x2: f32, y: f32, color: Color) {
		let line = Path::line(Point::new(x1, y), Point::new(x2, y));
		frame.stroke(&line, Stroke::default().with_color(color).with_width(1.0));
	}

	fn draw_vertical_line(&self, frame: &mut canvas::Frame, x: f32, y1: f32, y2: f32, color: Color) {
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
	fn clip_base_color(track_index: usize, _clip_id: usize) -> Color {
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

		PALETTE[track_index % PALETTE.len()]
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
