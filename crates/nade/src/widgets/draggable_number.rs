use constants::style::{
	BORDER_ACTIVE_COLOR, BORDER_RADIUS, BORDER_THICKNESS, FONT_LABEL, INPUT_HEIGHT, SPACE_1, SPACE_2, SPACE_4,
	TEXT_PRIMARY_COLOR, WIDGET_COLOR,
};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Widget};
use iced::advanced::{Clipboard, Shell, mouse};
use iced::event::Event;
use iced::keyboard;
use iced::mouse::Cursor;
use iced::widget::text_input;
use iced::{Border, Element, Length, Point, Rectangle, Size};

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

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> for DraggableNumber<'a, Message, Theme, Renderer>
where
	Message: Clone,
	Renderer: iced::advanced::text::Renderer,
	Theme: iced::widget::text::Catalog + iced::widget::text_input::Catalog,
{
	fn size(&self) -> Size<Length> {
		Size {
			width: self.width,
			height: Length::Fixed(INPUT_HEIGHT),
		}
	}

	fn layout(&mut self, tree: &mut widget::Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
		let state = tree.state.downcast_ref::<State>();

		if state.mode == Mode::Editing {
			let text_value = state.text_value.clone();
			let mut text_input: text_input::TextInput<Message, Theme, Renderer> =
				text_input::TextInput::new("", &text_value)
					.padding([SPACE_1 as u16, SPACE_2 as u16])
					.size(FONT_LABEL);
			Widget::layout(&mut text_input, tree, renderer, limits)
		} else {
			let size = limits.resolve(
				self.width,
				Length::Fixed(INPUT_HEIGHT),
				Size::new(SPACE_4 * 6.0, INPUT_HEIGHT),
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
				text_input::TextInput::new("", &state.text_value)
					.padding([SPACE_1 as u16, SPACE_2 as u16])
					.size(FONT_LABEL);
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
						radius: BORDER_RADIUS.into(),
						width: BORDER_THICKNESS,
						color: BORDER_ACTIVE_COLOR,
					},
					..Default::default()
				},
				WIDGET_COLOR,
			);

			let content = format!("{:.2}", self.value);

			renderer.fill_text(
				iced::advanced::text::Text {
					content,
					bounds: Size::new(bounds.width, bounds.height),
					size: FONT_LABEL.into(),
					line_height: iced::advanced::text::LineHeight::Relative(1.2),
					font: renderer.default_font(),
					align_x: iced::alignment::Horizontal::Center.into(),
					align_y: iced::alignment::Vertical::Center,
					wrapping: iced::advanced::text::Wrapping::Word,
					shaping: iced::advanced::text::Shaping::Basic,
				},
				bounds.center(),
				TEXT_PRIMARY_COLOR,
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
		tree.diff_children(&[Element::<Message, Theme, Renderer>::new(text_input::TextInput::new(
			"", "",
		))]);
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
					if (position.y - start_pos.y).abs() > 2.0 {
						state.mode = Mode::Dragging {
							start_pos,
							start_value: self.value,
						};
						shell.capture_event();
					}
				}
				Mode::Dragging { start_pos, start_value } => {
					let delta = start_pos.y - position.y;
					let new_val = start_value + delta * self.step;
					let new_val = (new_val * 1000.0).round() / 1000.0;

					if (new_val - self.value).abs() > f32::EPSILON {
						shell.publish((self.on_change)(new_val));
					}

					shell.capture_event();
				}
				_ => {}
			},
			Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => match state.mode {
				Mode::PotentialDrag { .. } => {
					if let Some(cursor_position) = cursor.position()
						&& bounds.contains(cursor_position)
						&& keyboard::Modifiers::default().command()
					{
						shell.publish((self.on_change)(self.default_value));
					}
					state.mode = Mode::Idle;
					shell.capture_event();
				}
				Mode::Dragging { .. } => {
					state.mode = Mode::Idle;
					shell.capture_event();
				}
				_ => {}
			},
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
