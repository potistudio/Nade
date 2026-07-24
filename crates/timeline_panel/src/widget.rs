//! タイムラインウィジェットの実装

use iced::{
	Color, Element, Event, Length, Point, Rectangle, Size, Theme,
	keyboard::{self, Key},
	mouse,
	widget::{
		button,
		canvas::{self, Canvas, Geometry, Path, Stroke, Text},
		column, container, row, slider,
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
		use constants::style::{FONT_UI, PAD_BUTTON, PAD_TOOLBAR, SPACE_2, TOOLBAR_HEIGHT};
		use constants::widgets;

		let zoom_percent = (time_scale * 100.0) as i32;

		let zoom_out_btn = button(widgets::button_body_center(widgets::ui_label("−", FONT_UI)))
			.on_press(TimelineMessage::ZoomOut)
			.padding(PAD_BUTTON)
			.width(Length::Shrink)
			.height(TOOLBAR_HEIGHT)
			.style(widgets::button_tool);

		let zoom_slider = slider(0.1..=10.0, time_scale, TimelineMessage::ZoomChanged)
			.step(0.1_f32)
			.width(120)
			.style(widgets::slider);

		let zoom_in_btn = button(widgets::button_body_center(widgets::ui_label("+", FONT_UI)))
			.on_press(TimelineMessage::ZoomIn)
			.padding(PAD_BUTTON)
			.width(Length::Shrink)
			.height(TOOLBAR_HEIGHT)
			.style(widgets::button_tool);

		let zoom_label = widgets::ui_label(format!("{zoom_percent}%"), FONT_UI).width(36);

		let reset_btn = button(widgets::button_body_center(widgets::ui_label("1:1", FONT_UI)))
			.on_press(TimelineMessage::ResetZoom)
			.padding(PAD_BUTTON)
			.width(Length::Shrink)
			.height(TOOLBAR_HEIGHT)
			.style(widgets::button_tool);

		let controls = row![zoom_out_btn, zoom_slider, zoom_in_btn, zoom_label, reset_btn,]
			.spacing(SPACE_2)
			.align_y(iced::Alignment::Center);

		container(controls)
			.padding(PAD_TOOLBAR)
			.width(Length::Fill)
			.style(widgets::toolbar)
			.into()
	}

	fn draw_timeline(&self, frame: &mut canvas::Frame, bounds: Size) {
		let layout = TimelineLayout::from_bounds(Rectangle::new(Point::ORIGIN, bounds));

		frame.fill_rectangle(Point::ORIGIN, bounds, colors::BACKGROUND);

		self.draw_range_slider(frame, self.state, self.model, layout.range_slider);
		self.draw_timeline_content(frame, self.state, self.model, layout.content);
		self.draw_ruler(frame, self.state, layout.ruler);
		self.draw_track_labels(frame, self.state, self.model, layout.track_labels);
		self.draw_playhead(frame, self.state, layout.ruler, layout.content);
		self.draw_selection_rect(frame);
	}

	fn draw_range_slider(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		model: &TimelineModel,
		rect: Rectangle,
	) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::RANGE_SLIDER_BG);
		self.draw_horizontal_line(
			frame,
			rect.x,
			rect.x + rect.width,
			rect.y + rect.height - 1.0,
			colors::BORDER_DARK,
		);

		let padding = 3.0_f32;
		let track_rect = Rectangle {
			x: rect.x + padding,
			y: rect.y + padding,
			width: (rect.width - padding * 2.0).max(0.0),
			height: rect.height - padding * 2.0,
		};

		let track_path = Path::rounded_rectangle(
			track_rect.position(),
			track_rect.size(),
			(track_rect.height / 2.0).into(),
		);
		frame.fill(&track_path, colors::RANGE_SLIDER_TRACK);

		let total_duration = state.total_timeline_duration(model);
		if total_duration <= 0.0 || track_rect.width <= 0.0 {
			return;
		}

		let visible_start = state.visible_start_time();
		let visible_end = state.visible_end_time();

		let norm_start = (visible_start / total_duration).clamp(0.0, 1.0);
		let norm_end = (visible_end / total_duration).clamp(0.0, 1.0);

		let handle_l = track_rect.x + norm_start * track_rect.width;
		let handle_r = (track_rect.x + norm_end * track_rect.width).max(handle_l + 4.0);
		let handle_w = handle_r - handle_l;

		let handle_rect = Rectangle {
			x: handle_l,
			y: track_rect.y,
			width: handle_w,
			height: track_rect.height,
		};

		let handle_path = Path::rounded_rectangle(
			handle_rect.position(),
			handle_rect.size(),
			(track_rect.height / 2.0).into(),
		);
		frame.fill(&handle_path, colors::RANGE_SLIDER_HANDLE);
		frame.stroke(
			&handle_path,
			Stroke::default()
				.with_color(colors::RANGE_SLIDER_HANDLE_BORDER)
				.with_width(1.0),
		);
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

		frame.fill_rectangle(local_rect.position(), local_rect.size(), colors::SELECTION_FILL);

		let sel_path = Path::rectangle(local_rect.position(), local_rect.size());
		frame.stroke(
			&sel_path,
			Stroke::default().with_color(colors::SELECTION_STROKE).with_width(1.0),
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
			let handle_width = TRACK_HANDLE_WIDTH;
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

			let rendered = model.is_track_rendered(i);
			// V / S は実状態そのものを表示（見た目だけオフにしない）
			self.draw_track_ctrl_btn(frame, track_visible_btn_rect(rect.x, y), "V", track.visible, true);
			self.draw_track_ctrl_btn(
				frame,
				track_solo_btn_rect(rect.x, y),
				"S",
				track.solo,
				track.visible || track.solo,
			);

			let text_color = if rendered {
				colors::TEXT_PRIMARY
			} else {
				colors::TEXT_MUTED
			};
			frame.fill_text(Text {
				content: track.name.clone(),
				position: Point::new(track_name_x(rect.x), y + TRACK_HEIGHT / 2.0 - 6.0),
				color: text_color,
				size: 11.0.into(),
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

		// Brief highlight for track that was just reordered
		if let Some((track_idx, alpha)) = self.glowing_track {
			let glow_y = rect.y + (track_idx as f32 * TRACK_HEIGHT) + state.scroll_offset.y;
			let glow_color = Color {
				a: alpha * 0.25,
				..colors::SELECTION
			};

			frame.fill_rectangle(
				Point::new(rect.x, glow_y),
				Size::new(rect.width, TRACK_HEIGHT),
				glow_color,
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
			width: x_end - x_start,
			height: TRACK_HEIGHT - TRACK_PADDING * 2.0,
		};

		let is_selected = state.is_clip_selected(track_index, clip.id);
		let is_hovered = state.is_clip_hovered(track_index, clip.id);
		let is_shrinking = state.alt_shrinking_clip == Some((track_index, clip.id));
		// Duplicate clips (id >= 10000) are shown semi-transparent during drag
		let is_duplicate = clip.id >= 10000;
		let base_color = Self::clip_base_color(track_index, clip.id);
		let mut clip_color = if is_selected {
			lighten_color(base_color, 0.2)
		} else {
			base_color
		};
		if !track_rendered {
			clip_color.a *= 0.35;
		}

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

	fn draw_track_ctrl_btn(
		&self,
		frame: &mut canvas::Frame,
		rect: Rectangle,
		label: &str,
		active: bool,
		enabled: bool,
	) {
		let bg = if !enabled {
			Color::from_rgba(1.0, 1.0, 1.0, 0.03)
		} else if active {
			Color {
				a: 0.35,
				..colors::SELECTION
			}
		} else {
			Color::from_rgba(1.0, 1.0, 1.0, 0.06)
		};
		frame.fill_rectangle(rect.position(), rect.size(), bg);

		let border = if !enabled {
			Color::from_rgba(1.0, 1.0, 1.0, 0.06)
		} else if active {
			Color {
				a: 0.85,
				..colors::SELECTION
			}
		} else {
			colors::BORDER_SUBTLE
		};
		let path = Path::rectangle(rect.position(), rect.size());
		frame.stroke(&path, Stroke::default().with_color(border).with_width(1.0));

		let text_color = if !enabled {
			Color::from_rgba(colors::TEXT_MUTED.r, colors::TEXT_MUTED.g, colors::TEXT_MUTED.b, 0.35)
		} else if active {
			colors::TEXT_PRIMARY
		} else {
			colors::TEXT_MUTED
		};
		frame.fill_text(Text {
			content: label.to_string(),
			position: Point::new(rect.x + rect.width * 0.5 - 3.5, rect.y + 1.0),
			color: text_color,
			size: 10.0.into(),
			..Text::default()
		});
	}

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
