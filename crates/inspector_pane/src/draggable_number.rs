use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Widget};
use iced::advanced::{Clipboard, Shell, mouse};
use iced::event::Event;
use iced::keyboard;
use iced::mouse::Cursor;
use iced::widget::text_input;
use iced::{Border, Color, Element, Length, Point, Rectangle, Size};

pub struct DraggableNumber<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
	Renderer: iced::advanced::text::Renderer,
{
	value: f32,
	default_value: f32,
	step: f32,
	on_change: Box<dyn Fn(f32) -> Message + 'a>,
	width: Length,
	_marker: std::marker::PhantomData<(Theme, Renderer)>,
}

impl<'a, Message, Theme, Renderer> DraggableNumber<'a, Message, Theme, Renderer>
where
	Renderer: iced::advanced::text::Renderer,
{
	pub fn new(value: f32, default_value: f32, on_change: impl Fn(f32) -> Message + 'a) -> Self {
		Self {
			value,
			default_value,
			step: 0.1,
			on_change: Box::new(on_change),
			width: Length::Fill,
			_marker: std::marker::PhantomData,
		}
	}

	pub fn step(mut self, step: f32) -> Self {
		self.step = step;
		self
	}

	pub fn width(mut self, width: impl Into<Length>) -> Self {
		self.width = width.into();
		self
	}
}

pub fn draggable_number<'a, Message, Theme, Renderer>(
	value: f32,
	default_value: f32,
	on_change: impl Fn(f32) -> Message + 'a,
) -> DraggableNumber<'a, Message, Theme, Renderer>
where
	Renderer: iced::advanced::text::Renderer,
{
	DraggableNumber::new(value, default_value, on_change)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
	Idle,
	PotentialDrag { start_pos: Point },
	Dragging { start_pos: Point, start_value: f32 },
	Editing,
}

impl Default for Mode {
	fn default() -> Self {
		Self::Idle
	}
}

struct State {
	mode: Mode,
	text_value: String,
}

impl Default for State {
	fn default() -> Self {
		Self {
			mode: Mode::Idle,
			text_value: String::new(),
		}
	}
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
	for DraggableNumber<'a, Message, Theme, Renderer>
where
	Message: Clone,
	Renderer: iced::advanced::text::Renderer,
	Theme: iced::widget::text::Catalog + iced::widget::text_input::Catalog,
{
	fn size(&self) -> Size<Length> {
		Size {
			width: self.width,
			height: Length::Fixed(20.0),
		}
	}

	fn layout(
		&mut self,
		tree: &mut widget::Tree,
		renderer: &Renderer,
		limits: &layout::Limits,
	) -> layout::Node {
		let state = tree.state.downcast_ref::<State>();

		if state.mode == Mode::Editing {
			let mut text_input: text_input::TextInput<Message, Theme, Renderer> =
				text_input::TextInput::new("", &state.text_value);
			Widget::layout(&mut text_input, tree, renderer, limits)
		} else {
			let text_size = Size::new(50.0, 20.0);
			let padding_h = 10.0;
			let padding_v = 10.0;
			let size = limits.resolve(
				self.width,
				Length::Shrink,
				Size::new(text_size.width + padding_h, text_size.height + padding_v),
			);
			layout::Node::new(size)
		}
	}

	fn draw(
		&self,
		tree: &widget::Tree,
		renderer: &mut Renderer,
		theme: &Theme,
		style: &renderer::Style,
		layout: Layout<'_>,
		cursor: Cursor,
		viewport: &Rectangle,
	) {
		let state = tree.state.downcast_ref::<State>();

		if state.mode == Mode::Editing {
			let text_input: text_input::TextInput<Message, Theme, Renderer> =
				text_input::TextInput::new("", &state.text_value);
			Widget::draw(
				&text_input,
				&tree.children[0],
				renderer,
				theme,
				style,
				layout,
				cursor,
				viewport,
			);
		} else {
			let bounds = layout.bounds();

			renderer.fill_quad(
				renderer::Quad {
					bounds,
					border: Border {
						radius: 4.0.into(),
						width: 1.0,
						color: Color::from_rgb(0.3, 0.3, 0.3),
					},
					..Default::default()
				},
				Color::from_rgb(0.15, 0.15, 0.15),
			);

			let content = format!("{:.2}", self.value);

			renderer.fill_text(
				iced::advanced::text::Text {
					content,
					bounds: Size::new(bounds.width, bounds.height),
					size: 12.0.into(),
					line_height: iced::advanced::text::LineHeight::Relative(1.3),
					font: renderer.default_font(),
					align_x: iced::alignment::Horizontal::Center.into(),
					align_y: iced::alignment::Vertical::Center,
					wrapping: iced::advanced::text::Wrapping::Word,
					shaping: iced::advanced::text::Shaping::Basic,
				},
				bounds.center(),
				Color::WHITE,
				*viewport,
			);
		}
	}

	fn children(&self) -> Vec<widget::Tree> {
		vec![widget::Tree::new(Element::<Message, Theme, Renderer>::new(
			text_input::TextInput::new("", ""),
		))]
	}

	fn diff(&self, tree: &mut widget::Tree) {
		tree.diff_children(&[Element::<Message, Theme, Renderer>::new(
			text_input::TextInput::new("", ""),
		)]);
	}

	fn tag(&self) -> widget::tree::Tag {
		widget::tree::Tag::of::<State>()
	}

	fn state(&self) -> widget::tree::State {
		widget::tree::State::new(State::default())
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

		if state.mode == Mode::Editing {
			if let Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) = event
				&& let Some(cursor_position) = cursor.position()
				&& !bounds.contains(cursor_position)
			{
				state.mode = Mode::Idle;
				shell.capture_event();
			}
			return;
		}

		match event {
			Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
				if let Some(cursor_position) = cursor.position()
					&& bounds.contains(cursor_position)
					&& matches!(state.mode, Mode::Idle)
				{
					state.mode = Mode::PotentialDrag {
						start_pos: cursor_position,
					};
					shell.capture_event();
				}
			}
			Event::Mouse(mouse::Event::CursorMoved { position }) => match state.mode {
				Mode::PotentialDrag { start_pos } => {
					let delta = position.x - start_pos.x;
					if delta.abs() > 3.0 {
						state.mode = Mode::Dragging {
							start_pos,
							start_value: self.value,
						};
						shell.capture_event();
					}
				}
				Mode::Dragging {
					start_pos,
					start_value,
				} => {
					let delta = position.x - start_pos.x;
					let new_value = start_value + delta * self.step;
					shell.publish((self.on_change)(new_value));
					shell.capture_event();
				}
				_ => {}
			},
			Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => match state.mode {
				Mode::PotentialDrag { .. } => {
					state.mode = Mode::Editing;
					state.text_value = format!("{:.2}", self.value);
					shell.capture_event();
				}
				Mode::Dragging { .. } => {
					state.mode = Mode::Idle;
					shell.capture_event();
				}
				_ => {}
			},
			Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
				if state.mode == Mode::Idle {
					match key {
						keyboard::Key::Named(keyboard::key::Named::ArrowUp) => {
							shell.publish((self.on_change)(self.value + self.step));
							shell.capture_event();
						}
						keyboard::Key::Named(keyboard::key::Named::ArrowDown) => {
							shell.publish((self.on_change)(self.value - self.step));
							shell.capture_event();
						}
						keyboard::Key::Named(keyboard::key::Named::Enter) => {
							state.mode = Mode::Editing;
							state.text_value = format!("{:.2}", self.value);
							shell.capture_event();
						}
						keyboard::Key::Character(char) if char == "0" => {
							shell.publish((self.on_change)(self.default_value));
							shell.capture_event();
						}
						_ => {}
					}
				}
			}
			_ => {}
		}
	}
}

impl<'a, Message, Theme, Renderer> From<DraggableNumber<'a, Message, Theme, Renderer>>
	for Element<'a, Message, Theme, Renderer>
where
	Message: Clone + 'a,
	Renderer: iced::advanced::text::Renderer + 'a,
	Theme: iced::widget::text::Catalog + iced::widget::text_input::Catalog + 'a,
{
	fn from(widget: DraggableNumber<'a, Message, Theme, Renderer>) -> Self {
		Element::new(widget)
	}
}
