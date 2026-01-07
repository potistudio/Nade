use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use iced::Element;
use panel_system::PanelSystemMessage;
use timeline_pane::TimelineWidget;

pub fn view<'a>(
	timeline: &'a TimelineWidget,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	timeline
		.view()
		.map(AppPanelMessage::Timeline)
		.map(PanelSystemMessage::AppMessage)
}
