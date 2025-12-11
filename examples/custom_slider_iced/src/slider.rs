use iced::{
	Element, Length, Point, Rectangle, Theme,
	alignment::{Horizontal, Vertical},
	event::Status,
	mouse::{self, Cursor, Interaction},
	widget::{
		canvas::{self, Canvas, Frame, Geometry, Path, Program, Text},
		column,
	},
};
use std::ops::RangeInclusive;

// =============================================================================
// 定数
// =============================================================================

mod consts {
	use iced::Color;

	pub const SLIDER_HEIGHT: f32 = 24.0;
	pub const INPUT_HEIGHT: f32 = 28.0;
	pub const TRACK_HEIGHT: f32 = 4.0;
	pub const THUMB_RADIUS: f32 = 8.0;

	pub mod colors {
		use super::Color;

		pub const TRACK: Color = Color::from_rgb(0.25, 0.25, 0.25);
		pub const TRACK_FILLED: Color = Color::from_rgb(0.4, 0.6, 0.9);
		pub const THUMB: Color = Color::from_rgb(0.9, 0.9, 0.9);
		pub const THUMB_HOVER: Color = Color::from_rgb(1.0, 1.0, 1.0);
		pub const TEXT: Color = Color::from_rgb(0.8, 0.8, 0.8);
		pub const LABEL: Color = Color::from_rgb(0.6, 0.6, 0.6);
		pub const INPUT_BG: Color = Color::from_rgb(0.12, 0.12, 0.12);
		pub const INPUT_BORDER: Color = Color::from_rgb(0.3, 0.3, 0.3);
		pub const INPUT_BORDER_FOCUSED: Color = Color::from_rgb(0.4, 0.6, 0.9);
	}
}

use consts::*;

// =============================================================================
// NumericSlider
// =============================================================================

pub struct NumericSlider {
	value: f32,
	range: RangeInclusive<f32>,
	step: f32,
	label: Option<String>,
}

impl NumericSlider {
	pub fn new(value: f32, range: RangeInclusive<f32>) -> Self {
		Self {
			value,
			range,
			step: 1.0,
			label: None,
		}
	}

	pub fn label(mut self, label: impl Into<String>) -> Self {
		self.label = Some(label.into());
		self
	}

	pub fn step(mut self, step: f32) -> Self {
		self.step = step;
		self
	}

	pub fn view(self) -> Element<'static, NumericSliderMessage> {
		let input_field = NumericInputField::new(self.value, self.range.clone(), self.step);
		let slider_field = SliderField::new(self.value, self.range.clone());

		let mut content = column![].spacing(4);

		if let Some(label) = &self.label {
			content = content.push(
				iced::widget::text(label.clone())
					.size(12)
					.color(colors::LABEL),
			);
		}

		content = content.push(
			Canvas::new(input_field)
				.width(Length::Fill)
				.height(Length::Fixed(INPUT_HEIGHT)),
		);

		content = content.push(
			Canvas::new(slider_field)
				.width(Length::Fill)
				.height(Length::Fixed(SLIDER_HEIGHT)),
		);

		content.into()
	}
}

// =============================================================================
// メッセージ
// =============================================================================

#[derive(Debug, Clone)]
pub enum NumericSliderMessage {
	ValueChanged(f32),
}

// =============================================================================
// 数値入力フィールド（ドラッグで値変更）
// =============================================================================

#[derive(Debug, Clone)]
struct NumericInputField {
	value: f32,
	range: RangeInclusive<f32>,
	step: f32,
}

impl NumericInputField {
	fn new(value: f32, range: RangeInclusive<f32>, step: f32) -> Self {
		Self { value, range, step }
	}
}

#[derive(Debug, Clone, Default)]
struct NumericInputState {
	is_dragging: bool,
	drag_start_y: f32,
	drag_start_value: f32,
	is_hovered: bool,
}

impl Program<NumericSliderMessage> for NumericInputField {
	type State = NumericInputState;

	fn draw(
		&self,
		state: &Self::State,
		renderer: &iced::Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: Cursor,
	) -> Vec<Geometry> {
		let mut frame = Frame::new(renderer, bounds.size());

		// 背景
		let background = Path::rectangle(Point::ORIGIN, bounds.size());

		let border_color = if state.is_dragging || state.is_hovered {
			colors::INPUT_BORDER_FOCUSED
		} else {
			colors::INPUT_BORDER
		};

		frame.fill(&background, colors::INPUT_BG);
		frame.stroke(
			&background,
			canvas::Stroke::default()
				.with_color(border_color)
				.with_width(1.0),
		);

		// 値テキスト
		let value_text = Text {
			content: format!("{:.2}", self.value),
			position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
			color: colors::TEXT,
			size: iced::Pixels(14.0),
			horizontal_alignment: Horizontal::Center,
			vertical_alignment: Vertical::Center,
			..Default::default()
		};
		frame.fill_text(value_text);

		// ドラッグ中のヒント（上下矢印）
		if state.is_hovered && !state.is_dragging {
			let arrow_size = 6.0;
			let center_x = bounds.width / 2.0;

			// 上矢印
			let up_arrow = Path::new(|builder| {
				builder.move_to(Point::new(center_x, 4.0));
				builder.line_to(Point::new(center_x - arrow_size / 2.0, 4.0 + arrow_size));
				builder.line_to(Point::new(center_x + arrow_size / 2.0, 4.0 + arrow_size));
				builder.close();
			});
			frame.fill(&up_arrow, colors::LABEL);

			// 下矢印
			let down_arrow = Path::new(|builder| {
				builder.move_to(Point::new(center_x, bounds.height - 4.0));
				builder.line_to(Point::new(
					center_x - arrow_size / 2.0,
					bounds.height - 4.0 - arrow_size,
				));
				builder.line_to(Point::new(
					center_x + arrow_size / 2.0,
					bounds.height - 4.0 - arrow_size,
				));
				builder.close();
			});
			frame.fill(&down_arrow, colors::LABEL);
		}

		vec![frame.into_geometry()]
	}

	fn update(
		&self,
		state: &mut Self::State,
		event: canvas::Event,
		bounds: Rectangle,
		cursor: Cursor,
	) -> (Status, Option<NumericSliderMessage>) {
		let cursor_position = cursor.position_in(bounds);

		// ホバー状態の更新
		state.is_hovered = cursor_position.is_some();

		match event {
			canvas::Event::Mouse(mouse_event) => match mouse_event {
				mouse::Event::ButtonPressed(mouse::Button::Left) => {
					if let Some(pos) = cursor_position {
						state.is_dragging = true;
						state.drag_start_y = pos.y;
						state.drag_start_value = self.value;
						return (Status::Captured, None);
					}
				}
				mouse::Event::ButtonReleased(mouse::Button::Left) => {
					if state.is_dragging {
						state.is_dragging = false;
						return (Status::Captured, None);
					}
				}
				mouse::Event::CursorMoved { .. } => {
					if state.is_dragging {
						if let Some(pos) = cursor.position() {
							// 上にドラッグすると値が増える、下にドラッグすると値が減る
							let delta_y = state.drag_start_y - (pos.y - bounds.y);
							let sensitivity = 0.5; // ドラッグ感度
							let delta_value = delta_y * sensitivity * self.step;

							let new_value = (state.drag_start_value + delta_value)
								.clamp(*self.range.start(), *self.range.end());

							// ステップに合わせて丸める
							let new_value = (new_value / self.step).round() * self.step;

							return (
								Status::Captured,
								Some(NumericSliderMessage::ValueChanged(new_value)),
							);
						}
					}
				}
				_ => {}
			},
			_ => {}
		}

		(Status::Ignored, None)
	}

	fn mouse_interaction(
		&self,
		state: &Self::State,
		bounds: Rectangle,
		cursor: Cursor,
	) -> Interaction {
		if state.is_dragging {
			// ドラッグ中は縦方向リサイズカーソルを表示
			Interaction::ResizingVertically
		} else if cursor.position_in(bounds).is_some() {
			Interaction::ResizingVertically
		} else {
			Interaction::default()
		}
	}
}

// =============================================================================
// スライダーフィールド
// =============================================================================

#[derive(Debug, Clone)]
struct SliderField {
	value: f32,
	range: RangeInclusive<f32>,
}

impl SliderField {
	fn new(value: f32, range: RangeInclusive<f32>) -> Self {
		Self { value, range }
	}

	fn value_to_x(&self, bounds: Rectangle) -> f32 {
		let ratio = (self.value - self.range.start()) / (self.range.end() - self.range.start());
		let usable_width = bounds.width - THUMB_RADIUS * 2.0;
		THUMB_RADIUS + ratio * usable_width
	}

	fn x_to_value(&self, x: f32, bounds: Rectangle) -> f32 {
		let usable_width = bounds.width - THUMB_RADIUS * 2.0;
		let ratio = ((x - THUMB_RADIUS) / usable_width).clamp(0.0, 1.0);
		self.range.start() + ratio * (self.range.end() - self.range.start())
	}
}

#[derive(Debug, Clone, Default)]
struct SliderFieldState {
	is_dragging: bool,
	is_hovered: bool,
}

impl Program<NumericSliderMessage> for SliderField {
	type State = SliderFieldState;

	fn draw(
		&self,
		state: &Self::State,
		renderer: &iced::Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: Cursor,
	) -> Vec<Geometry> {
		let mut frame = Frame::new(renderer, bounds.size());

		let thumb_x = self.value_to_x(bounds);
		let track_y = bounds.height / 2.0;

		// トラック背景
		let track_bg = Path::new(|builder| {
			builder.move_to(Point::new(THUMB_RADIUS, track_y));
			builder.line_to(Point::new(bounds.width - THUMB_RADIUS, track_y));
		});
		frame.stroke(
			&track_bg,
			canvas::Stroke::default()
				.with_color(colors::TRACK)
				.with_width(TRACK_HEIGHT)
				.with_line_cap(canvas::LineCap::Round),
		);

		// トラック（塗りつぶし部分）
		let track_filled = Path::new(|builder| {
			builder.move_to(Point::new(THUMB_RADIUS, track_y));
			builder.line_to(Point::new(thumb_x, track_y));
		});
		frame.stroke(
			&track_filled,
			canvas::Stroke::default()
				.with_color(colors::TRACK_FILLED)
				.with_width(TRACK_HEIGHT)
				.with_line_cap(canvas::LineCap::Round),
		);

		// サム（つまみ）
		let thumb_color = if state.is_dragging || state.is_hovered {
			colors::THUMB_HOVER
		} else {
			colors::THUMB
		};
		let thumb = Path::circle(Point::new(thumb_x, track_y), THUMB_RADIUS);
		frame.fill(&thumb, thumb_color);

		vec![frame.into_geometry()]
	}

	fn update(
		&self,
		state: &mut Self::State,
		event: canvas::Event,
		bounds: Rectangle,
		cursor: Cursor,
	) -> (Status, Option<NumericSliderMessage>) {
		let cursor_position = cursor.position_in(bounds);

		// サムのホバー判定
		if let Some(pos) = cursor_position {
			let thumb_x = self.value_to_x(bounds);
			let thumb_y = bounds.height / 2.0;
			let distance = ((pos.x - thumb_x).powi(2) + (pos.y - thumb_y).powi(2)).sqrt();
			state.is_hovered = distance <= THUMB_RADIUS * 1.5;
		} else {
			state.is_hovered = false;
		}

		match event {
			canvas::Event::Mouse(mouse_event) => match mouse_event {
				mouse::Event::ButtonPressed(mouse::Button::Left) => {
					if cursor_position.is_some() {
						state.is_dragging = true;
						if let Some(pos) = cursor_position {
							let new_value = self.x_to_value(pos.x, bounds);
							return (
								Status::Captured,
								Some(NumericSliderMessage::ValueChanged(new_value)),
							);
						}
					}
				}
				mouse::Event::ButtonReleased(mouse::Button::Left) => {
					if state.is_dragging {
						state.is_dragging = false;
						return (Status::Captured, None);
					}
				}
				mouse::Event::CursorMoved { .. } => {
					if state.is_dragging {
						if let Some(pos) = cursor.position() {
							let local_x = pos.x - bounds.x;
							let new_value = self.x_to_value(local_x, bounds);
							return (
								Status::Captured,
								Some(NumericSliderMessage::ValueChanged(new_value)),
							);
						}
					}
				}
				_ => {}
			},
			_ => {}
		}

		(Status::Ignored, None)
	}

	fn mouse_interaction(
		&self,
		state: &Self::State,
		bounds: Rectangle,
		cursor: Cursor,
	) -> Interaction {
		if state.is_dragging {
			Interaction::Grabbing
		} else if cursor.position_in(bounds).is_some() {
			Interaction::Pointer
		} else {
			Interaction::default()
		}
	}
}
