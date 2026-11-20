use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use iced::Element;
use iced::time::Instant;
use inspector_panel::{InspectorTarget, InspectorUi};
use panel_system::PanelSystemMessage;

pub fn view<'a>(
	target: InspectorTarget<'a>,
	ui: &'a InspectorUi,
	now: Instant,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	inspector_panel::view(target, ui, now)
		.map(|message| PanelSystemMessage::AppMessage(AppPanelMessage::Inspector(message)))
}
