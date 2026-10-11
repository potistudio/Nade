//! Iced animation curve editor panel.

use constants::style::{
	ACCENT_BRIGHT_COLOR, ACCENT_COLOR, AXIS_X_COLOR, AXIS_Y_COLOR, AXIS_Z_COLOR, BORDER_SUBTLE_COLOR, FONT_TINY,
	FONT_UI, KEYFRAME_COLOR, LABEL_COLORS, SPACE_1, SPACE_3, SPACE_4, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR,
	TEXT_SECONDARY_COLOR, WELL_COLOR,
};
use core::{AnimationCurve, HandleSide, Interpolation, Transform};
use domain::{TransformAnimation, TransformProperty};
use iced::{
	Alignment, Border, Color, Element, Event, Font, Length, Point, Rectangle, Size, Theme, Vector, keyboard, mouse,
	widget::{
		Space, button,
		canvas::{self, Canvas, Geometry, LineCap, LineJoin, Path, Stroke, Text},
		column, container, row, tooltip,
	},
};

const SIDEBAR_WIDTH: f32 = 148.0;
const TOOLBAR_HEIGHT: f32 = 32.0;
const HIT_RADIUS: f32 = 8.0;
const KEY_RADIUS: f32 = 4.5;
const HANDLE_RADIUS: f32 = 3.5;
const CURVE_SAMPLES: usize = 48;

#[derive(Debug, Clone)]
pub enum CurveEditorMessage {
	SelectProperty(TransformProperty),
	AddKey,
	DeleteSelected,
	SetInterpolation(Interpolation),
	FrameAll,
	CanvasEvent(CurveCanvasEvent),
}

#[derive(Debug, Clone)]
pub enum CurveCanvasEvent {
	MousePressed {
		position: Point,
		button: mouse::Button,
		bounds: Size,
	},
	MouseReleased(mouse::Button),
	MouseMoved {
		position: Point,
		bounds: Size,
	},
	MouseWheelScrolled {
		position: Point,
		delta: mouse::ScrollDelta,
		bounds: Size,
	},
	DeletePressed,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CurveEditorUpdate {
	pub playhead_time: Option<f32>,
	pub curve_changed: bool,
}

#[derive(Debug, Clone)]
enum DragState {
	Key(u64),
	Handle(u64, HandleSide),
	Playhead,
	Pan(Point),
}

/// View and selection state. Animation data remains in the domain model.
#[derive(Debug, Clone)]
pub struct CurveEditorState {
	pub selected_property: TransformProperty,
	pub selected_key: Option<u64>,
	x_scale: f32,
	y_scale: f32,
	offset: Vector,
	viewport: Size,
	drag: Option<DragState>,
}

impl Default for CurveEditorState {
	fn default() -> Self {
		Self {
			selected_property: TransformProperty::PositionX,
			selected_key: None,
			x_scale: 100.0,
			y_scale: 1.0,
			offset: Vector::new(20.0, 0.0),
			viewport: Size::new(800.0, 360.0),
			drag: None,
		}
	}
}

impl CurveEditorState {
	pub fn apply_message(
		&mut self,
		animation: &mut TransformAnimation,
		base: &Transform,
		message: CurveEditorMessage,
		current_time: f32,
	) -> CurveEditorUpdate {
		let mut update = CurveEditorUpdate::default();
		match message {
			CurveEditorMessage::SelectProperty(property) => {
				self.selected_property = property;
				self.selected_key = None;
			}
			CurveEditorMessage::AddKey => {
				let curve = animation.curve_mut(self.selected_property);
				let value = curve.evaluate(self.selected_property.value(base), current_time);
				self.selected_key = Some(curve.add_key(current_time, value));
				update.curve_changed = true;
			}
			CurveEditorMessage::DeleteSelected => {
				if let Some(id) = self.selected_key.take() {
					update.curve_changed = animation.curve_mut(self.selected_property).remove_key(id);
				}
			}
			CurveEditorMessage::SetInterpolation(interpolation) => {
				if let Some(id) = self.selected_key {
					update.curve_changed = animation
						.curve_mut(self.selected_property)
						.set_interpolation(id, interpolation);
				}
			}
			CurveEditorMessage::FrameAll => self.frame_all(animation),
			CurveEditorMessage::CanvasEvent(event) => {
				update = self.apply_canvas_event(animation, event, current_time);
			}
		}
		update
	}

	fn apply_canvas_event(
		&mut self,
		animation: &mut TransformAnimation,
		event: CurveCanvasEvent,
		current_time: f32,
	) -> CurveEditorUpdate {
		let mut update = CurveEditorUpdate::default();
		match event {
			CurveCanvasEvent::MousePressed {
				position,
				button,
				bounds,
			} => {
				self.viewport = bounds;
				match button {
					mouse::Button::Middle => self.drag = Some(DragState::Pan(position)),
					mouse::Button::Left => {
						let curve = animation.curve(self.selected_property);
						if let Some((id, side)) = self.hit_handle(curve, position) {
							self.selected_key = Some(id);
							self.drag = Some(DragState::Handle(id, side));
						} else if let Some(id) = self.hit_key(curve, position) {
							self.selected_key = Some(id);
							self.drag = Some(DragState::Key(id));
						} else if (self.time_to_x(current_time) - position.x).abs() <= HIT_RADIUS {
							self.drag = Some(DragState::Playhead);
						} else {
							self.selected_key = None;
							let time = self.x_to_time(position.x).max(0.0);
							self.drag = Some(DragState::Playhead);
							update.playhead_time = Some(time);
						}
					}
					_ => {}
				}
			}
			CurveCanvasEvent::MouseReleased(button) => {
				let ends_drag = matches!(
					(button, &self.drag),
					(
						mouse::Button::Left,
						Some(DragState::Key(_) | DragState::Handle(_, _) | DragState::Playhead)
					) | (mouse::Button::Middle, Some(DragState::Pan(_)))
				);
				if ends_drag {
					self.drag = None;
				}
			}
			CurveCanvasEvent::MouseMoved { position, bounds } => {
				self.viewport = bounds;
				match self.drag.clone() {
					Some(DragState::Key(id)) => {
						let [time, value] = self.screen_to_curve(position);
						update.curve_changed =
							animation
								.curve_mut(self.selected_property)
								.set_key(id, time.max(0.0), value);
					}
					Some(DragState::Handle(id, side)) => {
						let [time, value] = self.screen_to_curve(position);
						update.curve_changed = animation
							.curve_mut(self.selected_property)
							.set_handle(id, side, time, value);
					}
					Some(DragState::Playhead) => {
						update.playhead_time = Some(self.x_to_time(position.x).max(0.0));
					}
					Some(DragState::Pan(previous)) => {
						self.offset.x += position.x - previous.x;
						self.offset.y += position.y - previous.y;
						self.drag = Some(DragState::Pan(position));
					}
					None => {}
				}
			}
			CurveCanvasEvent::MouseWheelScrolled {
				position,
				delta,
				bounds,
			} => {
				self.viewport = bounds;
				let amount = match delta {
					mouse::ScrollDelta::Lines { y, .. } => y,
					mouse::ScrollDelta::Pixels { y, .. } => y / 30.0,
				};
				self.zoom_at(position, amount);
			}
			CurveCanvasEvent::DeletePressed => {
				if let Some(id) = self.selected_key.take() {
					update.curve_changed = animation.curve_mut(self.selected_property).remove_key(id);
				}
			}
		}
		update
	}

	fn frame_all(&mut self, animation: &TransformAnimation) {
		let mut bounds: Option<([f32; 2], [f32; 2])> = None;
		for property in TransformProperty::ALL {
			if let Some((min, max)) = animation.curve(property).bounds() {
				bounds = Some(match bounds {
					Some((all_min, all_max)) => (
						[all_min[0].min(min[0]), all_min[1].min(min[1])],
						[all_max[0].max(max[0]), all_max[1].max(max[1])],
					),
					None => (min, max),
				});
			}
		}
		let Some((min, max)) = bounds else {
			return;
		};
		let width = (self.viewport.width - 40.0).max(100.0);
		let height = (self.viewport.height - 40.0).max(100.0);
		self.x_scale = (width / (max[0] - min[0]).max(1.0)).clamp(10.0, 1000.0);
		self.y_scale = (height / (max[1] - min[1]).max(1.0)).clamp(0.01, 1000.0);
		self.offset.x = 20.0 - min[0] * self.x_scale;
		self.offset.y = (min[1] + max[1]) * 0.5 * self.y_scale;
	}

	fn zoom_at(&mut self, cursor: Point, amount: f32) {
		let before = self.screen_to_curve(cursor);
		let factor = (1.0 + amount * 0.12).clamp(0.25, 4.0);
		self.x_scale = (self.x_scale * factor).clamp(5.0, 5000.0);
		self.y_scale = (self.y_scale * factor).clamp(0.005, 5000.0);
		let after = self.curve_to_screen(before[0], before[1]);
		self.offset.x += cursor.x - after.x;
		self.offset.y += cursor.y - after.y;
	}

	fn time_to_x(&self, time: f32) -> f32 {
		time * self.x_scale + self.offset.x
	}

	fn x_to_time(&self, x: f32) -> f32 {
		(x - self.offset.x) / self.x_scale
	}

	fn curve_to_screen(&self, time: f32, value: f32) -> Point {
		Point::new(
			self.time_to_x(time),
			self.viewport.height * 0.5 + self.offset.y - value * self.y_scale,
		)
	}

	fn screen_to_curve(&self, point: Point) -> [f32; 2] {
		[
			self.x_to_time(point.x),
			(self.viewport.height * 0.5 + self.offset.y - point.y) / self.y_scale,
		]
	}

	fn hit_key(&self, curve: &AnimationCurve, point: Point) -> Option<u64> {
		curve
			.keys()
			.iter()
			.find(|key| distance(self.curve_to_screen(key.time, key.value), point) <= HIT_RADIUS)
			.map(|key| key.id)
	}

	fn hit_handle(&self, curve: &AnimationCurve, point: Point) -> Option<(u64, HandleSide)> {
		let id = self.selected_key?;
		let key = curve.key(id)?;
		[HandleSide::Left, HandleSide::Right].into_iter().find_map(|side| {
			let handle = key.handle_position(side);
			(distance(self.curve_to_screen(handle[0], handle[1]), point) <= HIT_RADIUS).then_some((id, side))
		})
	}
}

pub struct CurveEditorWidget<'a> {
	state: &'a CurveEditorState,
	animation: &'a TransformAnimation,
	current_time: f32,
}

impl<'a> CurveEditorWidget<'a> {
	pub fn new(state: &'a CurveEditorState, animation: &'a TransformAnimation, current_time: f32) -> Self {
		Self {
			state,
			animation,
			current_time,
		}
	}

	pub fn view(self) -> Element<'a, CurveEditorMessage> {
		use constants::icons::{Icon, icon};
		use constants::widgets::{self, button_body, button_body_center, mono_label, ui_label};

		let sidebar = EDITABLE_PROPERTIES
			.into_iter()
			.fold(column![].spacing(1), |column, property| {
				let selected = property == self.state.selected_property;
				let keys = self.animation.curve(property).keys().len();
				let dot = container(Space::new().width(8).height(8)).style(move |_| container::Style {
					background: Some(property_color(property).into()),
					border: Border {
						radius: 4.0.into(),
						..Default::default()
					},
					..Default::default()
				});
				let mut content = row![
					dot,
					ui_label(property.label(), FONT_UI).color(if selected {
						TEXT_PRIMARY_COLOR
					} else {
						TEXT_SECONDARY_COLOR
					}),
					Space::new().width(Length::Fill),
				]
				.spacing(SPACE_3 + 1.0)
				.align_y(Alignment::Center);
				if keys > 0 {
					content = content.push(
						row![
							icon(Icon::KeyframeFilled, 8.0, KEYFRAME_COLOR),
							mono_label(keys.to_string(), FONT_TINY).color(TEXT_MUTED_COLOR),
						]
						.spacing(SPACE_1 + 1.0)
						.align_y(Alignment::Center),
					);
				}
				column.push(
					button(button_body(content))
						.padding([0.0, SPACE_3 + 2.0])
						.width(Length::Fill)
						.height(24)
						.style(widgets::button_toggle(selected))
						.on_press(CurveEditorMessage::SelectProperty(property)),
				)
			});

		let interpolation = self
			.state
			.selected_key
			.and_then(|id| self.animation.curve(self.state.selected_property).key(id))
			.map(|key| key.interpolation);
		let has_selection = self.state.selected_key.is_some();

		let icon_button = |glyph: Icon, label: &'static str, active: bool, message: Option<CurveEditorMessage>| {
			let color = if active {
				ACCENT_BRIGHT_COLOR
			} else if message.is_some() {
				TEXT_SECONDARY_COLOR
			} else {
				Color {
					a: 0.35,
					..TEXT_MUTED_COLOR
				}
			};
			widgets::tip(
				button(button_body_center(icon(glyph, 14.0, color)))
					.padding(0)
					.width(28)
					.height(24)
					.style(widgets::button_toggle(active))
					.on_press_maybe(message),
				label,
				tooltip::Position::Bottom,
			)
		};
		let interpolation_button = |glyph, label, value: Interpolation| {
			icon_button(
				glyph,
				label,
				interpolation == Some(value),
				has_selection.then_some(CurveEditorMessage::SetInterpolation(value)),
			)
		};

		let selected_name = self.state.selected_property.label();
		let toolbar = row![
			ui_label(selected_name, FONT_UI).color(TEXT_PRIMARY_COLOR),
			Space::new().width(Length::Fill),
			icon_button(
				Icon::Keyframe,
				"Add Key at Playhead",
				false,
				Some(CurveEditorMessage::AddKey)
			),
			icon_button(
				Icon::Trash,
				"Delete Key  ⌫",
				false,
				has_selection.then_some(CurveEditorMessage::DeleteSelected)
			),
			widgets::vertical_separator(14.0),
			interpolation_button(Icon::InterpBezier, "Bezier", Interpolation::Bezier),
			interpolation_button(Icon::InterpLinear, "Linear", Interpolation::Linear),
			interpolation_button(Icon::InterpHold, "Hold", Interpolation::Hold),
			widgets::vertical_separator(14.0),
			icon_button(Icon::Fit, "Frame All", false, Some(CurveEditorMessage::FrameAll)),
		]
		.spacing(SPACE_1)
		.align_y(Alignment::Center)
		.padding([0.0, SPACE_4])
		.height(TOOLBAR_HEIGHT);

		let canvas: Element<'a, CurveEditorMessage> = Canvas::new(self).width(Length::Fill).height(Length::Fill).into();

		row![
			container(sidebar)
				.width(SIDEBAR_WIDTH)
				.height(Length::Fill)
				.padding([SPACE_3, SPACE_3]),
			container(Space::new().width(1).height(Length::Fill)).style(|_| container::Style {
				background: Some(BORDER_SUBTLE_COLOR.into()),
				..Default::default()
			}),
			column![toolbar, canvas].width(Length::Fill).height(Length::Fill),
		]
		.into()
	}
}

impl std::fmt::Debug for CurveEditorWidget<'_> {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.debug_struct("CurveEditorWidget").finish_non_exhaustive()
	}
}

impl canvas::Program<CurveEditorMessage> for CurveEditorWidget<'_> {
	type State = ();

	fn draw(
		&self,
		_state: &Self::State,
		renderer: &iced::Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		let mut frame = canvas::Frame::new(renderer, bounds.size());
		frame.fill_rectangle(Point::ORIGIN, bounds.size(), WELL_COLOR);
		self.draw_grid(&mut frame, bounds.size());
		for property in TransformProperty::ALL {
			self.draw_curve(
				&mut frame,
				self.animation.curve(property),
				property_color(property),
				property == self.state.selected_property,
			);
		}
		self.draw_playhead(&mut frame, bounds.height);
		if self.animation.curve(self.state.selected_property).is_empty() {
			self.draw_empty_hint(&mut frame, bounds.size());
		}
		vec![frame.into_geometry()]
	}

	fn update(
		&self,
		_state: &mut Self::State,
		event: &Event,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> Option<canvas::Action<CurveEditorMessage>> {
		let position = cursor.position_in(bounds)?;
		match event {
			Event::Mouse(mouse::Event::ButtonPressed(button)) => Some(canvas::Action::publish(
				CurveEditorMessage::CanvasEvent(CurveCanvasEvent::MousePressed {
					position,
					button: *button,
					bounds: bounds.size(),
				}),
			)),
			Event::Mouse(mouse::Event::ButtonReleased(button)) => Some(canvas::Action::publish(
				CurveEditorMessage::CanvasEvent(CurveCanvasEvent::MouseReleased(*button)),
			)),
			Event::Mouse(mouse::Event::CursorMoved { .. }) => Some(canvas::Action::publish(
				CurveEditorMessage::CanvasEvent(CurveCanvasEvent::MouseMoved {
					position,
					bounds: bounds.size(),
				}),
			)),
			Event::Mouse(mouse::Event::WheelScrolled { delta }) => Some(canvas::Action::publish(
				CurveEditorMessage::CanvasEvent(CurveCanvasEvent::MouseWheelScrolled {
					position,
					delta: *delta,
					bounds: bounds.size(),
				}),
			)),
			Event::Keyboard(keyboard::Event::KeyPressed {
				key: keyboard::Key::Named(keyboard::key::Named::Delete | keyboard::key::Named::Backspace),
				..
			}) => Some(canvas::Action::publish(CurveEditorMessage::CanvasEvent(
				CurveCanvasEvent::DeletePressed,
			))),
			_ => None,
		}
	}

	fn mouse_interaction(&self, _state: &Self::State, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
		cursor
			.position_in(bounds)
			.map(|_| mouse::Interaction::Crosshair)
			.unwrap_or_default()
	}
}

impl CurveEditorWidget<'_> {
	fn draw_grid(&self, frame: &mut canvas::Frame, size: Size) {
		let time_step = grid_step(self.state.x_scale, 80.0);
		let value_step = grid_step(self.state.y_scale, 48.0);
		let grid = Stroke::default()
			.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.04))
			.with_width(1.0);

		let min_time = self.state.x_to_time(0.0).floor();
		let max_time = self.state.x_to_time(size.width).ceil();
		let mut time = (min_time / time_step).floor() * time_step;
		while time <= max_time {
			let x = self.state.time_to_x(time);
			frame.stroke(&Path::line(Point::new(x, 0.0), Point::new(x, size.height)), grid);
			frame.fill_text(Text {
				content: format!("{time:.1}s"),
				position: Point::new(x + 4.0, 4.0),
				color: TEXT_MUTED_COLOR,
				size: 9.5.into(),
				font: Font::MONOSPACE,
				..Text::default()
			});
			time += time_step;
		}

		let top_value = self.state.screen_to_curve(Point::new(0.0, 0.0))[1];
		let bottom_value = self.state.screen_to_curve(Point::new(0.0, size.height))[1];
		let mut value = (bottom_value / value_step).floor() * value_step;
		while value <= top_value {
			let y = self.state.curve_to_screen(0.0, value).y;
			let stroke = if value.abs() < value_step * 0.01 {
				Stroke::default()
					.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.12))
					.with_width(1.0)
			} else {
				grid
			};
			frame.stroke(&Path::line(Point::new(0.0, y), Point::new(size.width, y)), stroke);
			frame.fill_text(Text {
				content: format_value(value, value_step),
				position: Point::new(4.0, y - 13.0),
				color: TEXT_MUTED_COLOR,
				size: 9.5.into(),
				font: Font::MONOSPACE,
				..Text::default()
			});
			value += value_step;
		}
	}

	fn draw_curve(&self, frame: &mut canvas::Frame, curve: &AnimationCurve, color: Color, selected: bool) {
		if curve.keys().is_empty() {
			return;
		}
		if curve.keys().len() > 1 {
			let path = Path::new(|builder| {
				let first = curve.keys()[0];
				builder.move_to(self.state.curve_to_screen(first.time, first.value));
				for pair in curve.keys().windows(2) {
					let left = pair[0];
					let right = pair[1];
					match left.interpolation {
						Interpolation::Hold => {
							builder.line_to(self.state.curve_to_screen(right.time, left.value));
							builder.line_to(self.state.curve_to_screen(right.time, right.value));
						}
						Interpolation::Linear => builder.line_to(self.state.curve_to_screen(right.time, right.value)),
						Interpolation::Bezier => {
							for sample in 1..=CURVE_SAMPLES {
								let amount = sample as f32 / CURVE_SAMPLES as f32;
								let time = left.time + (right.time - left.time) * amount;
								builder.line_to(self.state.curve_to_screen(time, curve.evaluate(left.value, time)));
							}
						}
					}
				}
			});
			frame.stroke(
				&path,
				Stroke::default()
					.with_color(Color {
						a: if selected { 1.0 } else { 0.32 },
						..color
					})
					.with_width(if selected { 2.0 } else { 1.25 })
					.with_line_cap(LineCap::Round)
					.with_line_join(LineJoin::Round),
			);
		}

		// 選択キーのハンドル（キーより先に描いて下に潜らせる）
		if selected
			&& let Some(id) = self.state.selected_key
			&& let Some(key) = curve.key(id)
		{
			let anchor = self.state.curve_to_screen(key.time, key.value);
			for side in [HandleSide::Left, HandleSide::Right] {
				let handle = key.handle_position(side);
				let handle_position = self.state.curve_to_screen(handle[0], handle[1]);
				frame.stroke(
					&Path::line(anchor, handle_position),
					Stroke::default()
						.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.45))
						.with_width(1.0),
				);
				let knob = Path::circle(handle_position, HANDLE_RADIUS);
				frame.fill(&knob, Color::WHITE);
				frame.stroke(&knob, Stroke::default().with_color(ACCENT_COLOR).with_width(1.0));
			}
		}

		for key in curve.keys() {
			let position = self.state.curve_to_screen(key.time, key.value);
			let is_active = selected && self.state.selected_key == Some(key.id);
			let radius = if is_active { KEY_RADIUS + 1.5 } else { KEY_RADIUS };
			let diamond = Path::new(|builder| {
				builder.move_to(Point::new(position.x, position.y - radius));
				builder.line_to(Point::new(position.x + radius, position.y));
				builder.line_to(Point::new(position.x, position.y + radius));
				builder.line_to(Point::new(position.x - radius, position.y));
				builder.close();
			});
			let fill = if is_active {
				Color::WHITE
			} else if selected {
				color
			} else {
				Color { a: 0.45, ..color }
			};
			frame.fill(&diamond, fill);
			frame.stroke(
				&diamond,
				Stroke::default()
					.with_color(if is_active {
						ACCENT_COLOR
					} else {
						Color::from_rgba(0.0, 0.0, 0.0, 0.55)
					})
					.with_width(if is_active { 1.5 } else { 1.0 }),
			);
		}
	}

	fn draw_playhead(&self, frame: &mut canvas::Frame, height: f32) {
		let x = self.state.time_to_x(self.current_time);
		frame.stroke(
			&Path::line(Point::new(x, 0.0), Point::new(x, height)),
			Stroke::default().with_color(ACCENT_COLOR).with_width(1.5),
		);
		let head = Path::rounded_rectangle(Point::new(x - 5.0, 0.0), Size::new(10.0, 12.0), 3.0.into());
		frame.fill(&head, ACCENT_COLOR);
	}

	fn draw_empty_hint(&self, frame: &mut canvas::Frame, size: Size) {
		frame.fill_text(Text {
			content: "No keyframes on this channel — press ◇ to add one at the playhead".into(),
			position: Point::new(size.width * 0.5, size.height * 0.5),
			color: TEXT_MUTED_COLOR,
			size: 11.0.into(),
			align_x: iced::advanced::text::Alignment::Center,
			align_y: iced::alignment::Vertical::Center,
			..Text::default()
		});
	}
}

/// Channels editable on 2D layers, in panel order.
const EDITABLE_PROPERTIES: [TransformProperty; 6] = [
	TransformProperty::PositionX,
	TransformProperty::PositionY,
	TransformProperty::RotationZ,
	TransformProperty::ScaleX,
	TransformProperty::ScaleY,
	TransformProperty::Opacity,
];

fn format_value(value: f32, step: f32) -> String {
	if step >= 1.0 {
		format!("{value:.0}")
	} else if step >= 0.1 {
		format!("{value:.1}")
	} else {
		format!("{value:.2}")
	}
}

fn property_color(property: TransformProperty) -> Color {
	match property {
		TransformProperty::PositionX => AXIS_X_COLOR,
		TransformProperty::PositionY => AXIS_Y_COLOR,
		TransformProperty::PositionZ => AXIS_Z_COLOR,
		TransformProperty::RotationX | TransformProperty::RotationY => TEXT_SECONDARY_COLOR,
		TransformProperty::RotationZ => LABEL_COLORS[1],
		TransformProperty::ScaleX => LABEL_COLORS[3],
		TransformProperty::ScaleY => LABEL_COLORS[5],
		TransformProperty::ScaleZ => TEXT_SECONDARY_COLOR,
		TransformProperty::Opacity => Color::from_rgb8(220, 220, 228),
	}
}

fn grid_step(scale: f32, target_pixels: f32) -> f32 {
	let raw = target_pixels / scale.max(0.0001);
	let power = 10.0_f32.powf(raw.log10().floor());
	let normalized = raw / power;
	let factor = if normalized <= 1.0 {
		1.0
	} else if normalized <= 2.0 {
		2.0
	} else if normalized <= 5.0 {
		5.0
	} else {
		10.0
	};
	factor * power
}

fn distance(a: Point, b: Point) -> f32 {
	((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}
