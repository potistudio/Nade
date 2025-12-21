use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use crate::theme::panel::COMPOSITION_BG;
use crate::theme::{TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY};
use iced::widget::{column, container, text};
use iced::{Element, Length};
use panel_system::PanelSystemMessage;

pub fn view<'a>() -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	let content = column![
		text("Composition").size(14).color(TEXT_PRIMARY),
		text("└─ Layer 1").size(12).color(TEXT_SECONDARY),
		text("   └─ Effect: Sine Wave").size(11).color(TEXT_MUTED),
	]
	.spacing(5)
	.padding(10);

	container(content)
		.width(Length::Fill)
		.height(Length::Fill)
		.style(|_| container::Style {
			background: Some(COMPOSITION_BG.into()),
			..Default::default()
		})
		.into()
}
