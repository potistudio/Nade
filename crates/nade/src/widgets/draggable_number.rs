//! Scrubbable numeric field (After Effects "hot text" / Cavalry style).
//!
//! - Drag horizontally to scrub (Shift ×10, Option ×0.1).
//! - Click to type a value; Enter/Tab commits, Escape cancels, clicking
//!   elsewhere commits.
//! - Command-click resets to the default value.

use constants::style::{
	ACCENT_COLOR, BORDER_RADIUS, FONT_LABEL, INPUT_HEIGHT, KEYFRAME_COLOR, TEXT_PRIMARY_COLOR, WELL_COLOR,
	WIDGET_COLOR, WIDGET_HOVER_COLOR, mix,
};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::text::{self as advanced_text};
use iced::advanced::widget::{self, Widget};
use iced::advanced::{Clipboard, Shell, mouse};
use iced::event::Event;
use iced::keyboard::{self, key::Named};
use iced::mouse::Cursor;
use iced::{Border, Color, Element, Length, Pixels, Point, Rectangle, Size};

/// Keyframe state of the property behind a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyState {
	#[default]
	Static,
	/// Animated, but no key at the current time.
	Animated,
	/// A key sits on the current time.
	OnKey,
}

const DRAG_THRESHOLD: f32 = 2.0;
const TEXT_PADDING: f32 = 7.0;

pub struct NumberField<'a, Message> {
	value: f32,
	default_value: f32,
	step: f32,
	precision: usize,
	scale: f32,
	suffix: &'static str,
	prefix: Option<(&'static str, Color)>,
	key_state: KeyState,
	on_change: Box<dyn Fn(f32) -> Message + 'a>,
	width: Length,
}

impl<'a, Message> NumberField<'a, Message> {
	pub fn new(value: f32, on_change: impl Fn(f32) -> Message + 'a) -> Self {
		Self {
			value,
			default_value: 0.0,
			step: 0.1,
			precision: 2,
			scale: 1.0,
			suffix: "",
			prefix: None,
			key_state: KeyState::Static,
			on_change: Box::new(on_change),
			width: Length::Fill,
		}
	}

	/// Value restored by Command-click.
	pub fn default_value(mut self, value: f32) -> Self {
		self.default_value = value;
		self
	}

	/// Change in value per scrubbed pixel.
	pub fn step(mut self, step: f32) -> Self {
		self.step = step;
		self
	}

	pub fn precision(mut self, precision: usize) -> Self {
		self.precision = precision;
		self
	}

	/// Display multiplier (e.g. `100.0` to show `0.85` as `85`).
	pub fn scale(mut self, scale: f32) -> Self {
		self.scale = scale;
		self
	}

	pub fn suffix(mut self, suffix: &'static str) -> Self {
		self.suffix = suffix;
		self
	}

	/// Inline axis label drawn at the left edge.
	pub fn prefix(mut self, label: &'static str, color: Color) -> Self {
		self.prefix = Some((label, color));
		self
	}

	pub fn key_state(mut self, key_state: KeyState) -> Self {
		self.key_state = key_state;
		self
	}

	pub fn width(mut self, width: impl Into<Length>) -> Self {
		self.width = width.into();
		self
	}

	fn display(&self) -> String {
		format!("{:.*}{}", self.precision, self.value * self.scale, self.suffix)
	}

	fn draft(&self) -> String {
		let shown = format!("{:.*}", self.precision, self.value * self.scale);
		// 末尾の 0 を落として編集しやすくする
		if shown.contains('.') {
			shown.trim_end_matches('0').trim_end_matches('.').to_string()
		} else {
			shown
		}
	}
}

pub fn number_field<'a, Message>(value: f32, on_change: impl Fn(f32) -> Message + 'a) -> NumberField<'a, Message> {
	NumberField::new(value, on_change)
}

#[derive(Debug, Clone, PartialEq, Default)]
enum Mode {
	#[default]
	Idle,
	Pressed {
		start: Point,
	},
	Scrubbing {
		start_x: f32,
		start_value: f32,
	},
	Editing {
		draft: String,
		/// The whole draft is selected; typing replaces it.
		replace: bool,
	},
}

#[derive(Debug, Default)]
struct State {
	mode: Mode,
	modifiers: keyboard::Modifiers,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for NumberField<'_, Message>
where
	Renderer: advanced_text::Renderer<Font = iced::Font>,
{
	fn size(&self) -> Size<Length> {
		Size {
			width: self.width,
			height: Length::Fixed(INPUT_HEIGHT),
		}
	}

	fn layout(&mut self, _tree: &mut widget::Tree, _renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
		layout::Node::new(limits.resolve(self.width, Length::Fixed(INPUT_HEIGHT), Size::new(48.0, INPUT_HEIGHT)))
	}

	fn tag(&self) -> widget::tree::Tag {
		widget::tree::Tag::of::<State>()
	}

	fn state(&self) -> widget::tree::State {
		widget::tree::State::new(State::default())
	}

	fn draw(
		&self,
		tree: &widget::Tree,
		renderer: &mut Renderer,
		_theme: &Theme,
		_style: &renderer::Style,
		layout: Layout<'_>,
		cursor: Cursor,
		viewport: &Rectangle,
	) {
		let state = tree.state.downcast_ref::<State>();
		let bounds = layout.bounds();
		let hovered = cursor.is_over(bounds);
		let editing = matches!(state.mode, Mode::Editing { .. });

		let mut background = match state.mode {
			Mode::Editing { .. } => WELL_COLOR,
			Mode::Scrubbing { .. } => WIDGET_HOVER_COLOR,
			_ if hovered => WIDGET_HOVER_COLOR,
			_ => WIDGET_COLOR,
		};
		if self.key_state == KeyState::OnKey && !editing {
			background = mix(background, KEYFRAME_COLOR, 0.14);
		}

		renderer.fill_quad(
			renderer::Quad {
				bounds,
				border: Border {
					radius: BORDER_RADIUS.into(),
					width: if editing { 1.0 } else { 0.0 },
					color: ACCENT_COLOR,
				},
				..Default::default()
			},
			background,
		);

		let font = renderer.default_font();
		let center_y = bounds.center_y();
		let mut value_left = bounds.x + TEXT_PADDING;

		if let Some((label, color)) = self.prefix {
			renderer.fill_text(
				text_spec(
					label.to_string(),
					bounds,
					FONT_LABEL - 1.0,
					advanced_text::Alignment::Left,
					font,
				),
				Point::new(bounds.x + TEXT_PADDING, center_y),
				color,
				*viewport,
			);
			value_left += 10.0;
		}

		let value_area = Rectangle {
			x: value_left,
			width: (bounds.x + bounds.width - TEXT_PADDING - value_left).max(0.0),
			..bounds
		};

		match &state.mode {
			Mode::Editing { draft, replace } => {
				let approx = draft.chars().count() as f32 * FONT_LABEL * 0.58;
				let text_right = value_area.x + value_area.width;
				if *replace && !draft.is_empty() {
					renderer.fill_quad(
						renderer::Quad {
							bounds: Rectangle {
								x: text_right - approx - 1.0,
								y: center_y - FONT_LABEL * 0.65,
								width: approx + 2.0,
								height: FONT_LABEL * 1.3,
							},
							border: Border {
								radius: 2.0.into(),
								..Default::default()
							},
							..Default::default()
						},
						Color {
							a: 0.45,
							..ACCENT_COLOR
						},
					);
				}
				renderer.fill_text(
					text_spec(
						draft.clone(),
						value_area,
						FONT_LABEL,
						advanced_text::Alignment::Right,
						font,
					),
					Point::new(text_right, center_y),
					TEXT_PRIMARY_COLOR,
					*viewport,
				);
				if !*replace {
					renderer.fill_quad(
						renderer::Quad {
							bounds: Rectangle {
								x: text_right + 1.0,
								y: center_y - FONT_LABEL * 0.6,
								width: 1.0,
								height: FONT_LABEL * 1.2,
							},
							..Default::default()
						},
						ACCENT_COLOR,
					);
				}
			}
			_ => {
				let color = match self.key_state {
					KeyState::Static => TEXT_PRIMARY_COLOR,
					KeyState::Animated | KeyState::OnKey => KEYFRAME_COLOR,
				};
				let (align, anchor_x) = if self.prefix.is_some() {
					(advanced_text::Alignment::Right, value_area.x + value_area.width)
				} else {
					(advanced_text::Alignment::Center, value_area.center_x())
				};
				renderer.fill_text(
					text_spec(self.display(), value_area, FONT_LABEL, align, font),
					Point::new(anchor_x, center_y),
					color,
					*viewport,
				);
			}
		}
	}

	fn update(
		&mut self,
		tree: &mut widget::Tree,
		event: &Event,
		layout: Layout<'_>,
		cursor: Cursor,
		_renderer: &Renderer,
		_clipboard: &mut dyn Clipboard,
		shell: &mut Shell<'_, Message>,
		_viewport: &Rectangle,
	) {
		let state = tree.state.downcast_mut::<State>();
		let bounds = layout.bounds();

		if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
			state.modifiers = *modifiers;
		}

		if let Mode::Editing { draft, replace } = &mut state.mode {
			match event {
				Event::Keyboard(keyboard::Event::KeyPressed { key, text, .. }) => {
					match key.as_ref() {
						keyboard::Key::Named(Named::Enter | Named::Tab) => {
							if let Ok(parsed) = draft.trim().parse::<f32>() {
								shell.publish((self.on_change)(parsed / self.scale));
							}
							state.mode = Mode::Idle;
						}
						keyboard::Key::Named(Named::Escape) => state.mode = Mode::Idle,
						keyboard::Key::Named(Named::Backspace) => {
							if *replace {
								draft.clear();
							} else {
								draft.pop();
							}
							*replace = false;
						}
						_ => {
							if let Some(text) = text {
								let accepted: String = text
									.chars()
									.filter(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
									.collect();
								if !accepted.is_empty() {
									if *replace {
										draft.clear();
									}
									draft.push_str(&accepted);
									*replace = false;
								}
							}
						}
					}
					shell.capture_event();
					shell.request_redraw();
				}
				Event::Keyboard(keyboard::Event::KeyReleased { .. }) => shell.capture_event(),
				Event::Mouse(mouse::Event::ButtonPressed(_)) if !cursor.is_over(bounds) => {
					if let Ok(parsed) = draft.trim().parse::<f32>() {
						shell.publish((self.on_change)(parsed / self.scale));
					}
					state.mode = Mode::Idle;
				}
				_ => {}
			}
			return;
		}

		match event {
			Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
				if let Some(position) = cursor.position_over(bounds) {
					state.mode = Mode::Pressed { start: position };
					shell.capture_event();
				}
			}
			Event::Mouse(mouse::Event::CursorMoved { position }) => match state.mode {
				Mode::Pressed { start } if (position.x - start.x).abs() > DRAG_THRESHOLD => {
					state.mode = Mode::Scrubbing {
						start_x: start.x,
						start_value: self.value,
					};
					shell.capture_event();
				}
				Mode::Scrubbing { start_x, start_value } => {
					let speed = if state.modifiers.shift() {
						10.0
					} else if state.modifiers.alt() {
						0.1
					} else {
						1.0
					};
					let next = start_value + (position.x - start_x) * self.step * speed;
					let next = (next * 1000.0).round() / 1000.0;
					if (next - self.value).abs() > f32::EPSILON {
						shell.publish((self.on_change)(next));
					}
					shell.capture_event();
				}
				_ => {}
			},
			Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => match state.mode {
				Mode::Pressed { .. } => {
					state.mode = if state.modifiers.command() {
						shell.publish((self.on_change)(self.default_value));
						Mode::Idle
					} else {
						Mode::Editing {
							draft: self.draft(),
							replace: true,
						}
					};
					shell.capture_event();
					shell.request_redraw();
				}
				Mode::Scrubbing { .. } => {
					state.mode = Mode::Idle;
					shell.capture_event();
				}
				_ => {}
			},
			_ => {}
		}
	}

	fn mouse_interaction(
		&self,
		tree: &widget::Tree,
		layout: Layout<'_>,
		cursor: Cursor,
		_viewport: &Rectangle,
		_renderer: &Renderer,
	) -> mouse::Interaction {
		let state = tree.state.downcast_ref::<State>();
		match state.mode {
			Mode::Scrubbing { .. } => mouse::Interaction::ResizingHorizontally,
			Mode::Editing { .. } if cursor.is_over(layout.bounds()) => mouse::Interaction::Text,
			_ if cursor.is_over(layout.bounds()) => mouse::Interaction::ResizingHorizontally,
			_ => mouse::Interaction::default(),
		}
	}
}

fn text_spec(
	content: String,
	bounds: Rectangle,
	size: f32,
	align_x: advanced_text::Alignment,
	font: iced::Font,
) -> advanced_text::Text {
	advanced_text::Text {
		content,
		// 幅で折り返さないよう十分に広く取る（位置合わせは align_x で行う）
		bounds: Size::new(bounds.width.max(1.0) + 512.0, bounds.height),
		size: Pixels(size),
		line_height: advanced_text::LineHeight::Relative(1.2),
		font,
		align_x,
		align_y: iced::alignment::Vertical::Center,
		shaping: advanced_text::Shaping::Basic,
		wrapping: advanced_text::Wrapping::None,
	}
}

impl<'a, Message, Theme, Renderer> From<NumberField<'a, Message>> for Element<'a, Message, Theme, Renderer>
where
	Message: 'a,
	Renderer: advanced_text::Renderer<Font = iced::Font> + 'a,
{
	fn from(widget: NumberField<'a, Message>) -> Self {
		Element::new(widget)
	}
}
