use iced::advanced::widget::{self, Widget};
use iced::advanced::{layout, renderer};
use iced::{Length, Padding, Rectangle, Size};

struct DummyWidget;

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for DummyWidget
where
	Renderer: iced::advanced::Renderer,
{
	fn size(&self) -> Size<Length> {
		Size::ZERO
	}

	fn layout(
		&mut self,
		_tree: &mut widget::Tree,
		_renderer: &Renderer,
		limits: &layout::Limits,
	) -> layout::Node {
		// Probe Limits::resolve signature (4 args expected)
		// Try passing a dummy 4th arg, see what type is expected
		let _ = limits.resolve(Length::Fill, Length::Fill, Size::ZERO, ());

		// Probe Padding fields
		let p = Padding::from(0.0);
		let _ = p.left; // Check if field exists
		let _ = p.horizontal(()); // Check argument need

		layout::Node::new(Size::ZERO)
	}

	fn draw(
		&self,
		_tree: &widget::Tree,
		_renderer: &mut Renderer,
		_theme: &Theme,
		_style: &renderer::Style,
		_layout: layout::Layout<'_>,
		_cursor: iced::advanced::mouse::Cursor,
		_viewport: &Rectangle,
	) {
	}

	fn children(&self) -> Vec<widget::Tree> {
		Vec::new()
	}

	fn diff(&self, _tree: &mut widget::Tree) {}
}
