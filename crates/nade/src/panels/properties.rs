use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use crate::theme::panel::PROPERTIES_BG;
use crate::theme::{TEXT_MUTED, TEXT_PRIMARY};
use iced::widget::{column, container, text};
use iced::{Element, Length};
use panel_system::PanelSystemMessage;

pub fn view<'a>() -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	let content = column![
		text("Properties").size(14).color(TEXT_PRIMARY),
		text("No selection").size(12).color(TEXT_MUTED),
	]
	.spacing(10)
	.padding(10);

	container(content)
		.width(Length::Fill)
		.height(Length::Fill)
		.style(|_| container::Style {
			background: Some(PROPERTIES_BG.into()),
			..Default::default()
		})
		.into()
}
