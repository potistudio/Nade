use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use crate::theme::TEXT_PRIMARY;
use crate::theme::panel::CONSOLE_BG;
use iced::widget::{column, container, text};
use iced::{Color, Element, Length};
use panel_system::PanelSystemMessage;

pub fn view<'a>() -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	let content = column![
		text("Console").size(14).color(TEXT_PRIMARY),
		text("[INFO] Application started")
			.size(11)
			.color(Color::from_rgb(0.7, 0.7, 0.7)),
	]
	.spacing(5)
	.padding(10);

	container(content)
		.width(Length::Fill)
		.height(Length::Fill)
		.style(|_| container::Style {
			background: Some(CONSOLE_BG.into()),
			..Default::default()
		})
		.into()
}
