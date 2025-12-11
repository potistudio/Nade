use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::{
	Widget,
	layout::{self, Layout},
	mouse, renderer,
	widget::Tree,
};
use iced::application::BootFn;
use iced::overlay::menu::State;
use iced::{Element, Size, Task, widget::text};

struct CustomSlider;

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for CustomSlider
where
	Renderer: renderer::Renderer,
{
	fn size(&self) -> Size<iced::Length> {
		Size::new(iced::Length::Fill, iced::Length::Fixed(40.0))
	}

	fn layout(
		&mut self,
		_tree: &mut Tree,
		_renderer: &Renderer,
		_limits: &layout::Limits,
	) -> layout::Node {
		layout::Node::new(Size::new(100., 50.))
	}

	fn draw(
		&self,
		_tree: &Tree,
		_renderer: &mut Renderer,
		_theme: &Theme,
		_style: &renderer::Style,
		_layout: Layout<'_>,
		_cursor: mouse::Cursor,
		_viewport: &iced::Rectangle,
	) {
		let bounds = _layout.bounds();
		let radius = 10.0;

		_renderer.fill_quad(
			renderer::Quad {
				bounds,
				border: iced::Border {
					radius: radius.into(),
					..Default::default()
				},
				shadow: iced::Shadow::default(),
				snap: true,
			},
			iced::Color::from_rgb(0.5, 0.5, 0.8),
		);
	}

	// fn on_event(
	// 	&mut self,
	// 	_state: &mut Tree,
	// 	_event: iced::Event,
	// 	_layout: Layout<'_>,
	// 	_cursor: mouse::Cursor,
	// 	_renderer: &Renderer,
	// 	_clipboard: &mut dyn iced::advanced::Clipboard,
	// 	_shell: &mut iced::advanced::Shell<'_, Message>,
	// 	_viewport: &iced::Rectangle,
	// ) -> iced_graphics::core::event::Status {
	// 	if let iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = _event {
	// 		if _cursor.is_over(_layout.bounds()) {
	// 			return iced_graphics::core::event::Status::Captured;
	// 		}
	// 	}
	// 	iced_graphics::core::event::Status::Ignored
	// }

	fn mouse_interaction(
		&self,
		_state: &Tree,
		layout: Layout<'_>,
		cursor: mouse::Cursor,
		_viewport: &iced::Rectangle,
		_renderer: &Renderer,
	) -> mouse::Interaction {
		let bounds = layout.bounds();
		if cursor
			.position()
			.map(|p| bounds.contains(p))
			.unwrap_or(false)
		{
			mouse::Interaction::Hidden
		} else {
			mouse::Interaction::default()
		}
	}
}

#[derive(Debug, Clone)]
enum Message {
	Increment,
}

#[derive(Default)]
struct App {
	counter: f32,
}

impl App {
	fn update(&mut self, _message: Message) {}

	fn view(&self) -> Element<'_, Message> {
		let slider: Element<'_, Message> = Element::new(CustomSlider);
		slider.into()
	}
}

fn main() -> iced::Result {
	unsafe {
		std::env::set_var("RUST_LOG", "debug");
	}
	env_logger::init();

	log::debug!("Starting Custom Slider Demo");

	iced::application(App::default, App::update, App::view).run()
}
