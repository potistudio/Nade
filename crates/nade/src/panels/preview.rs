use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use iced::Element;
use panel_system::PanelSystemMessage;
use viewer_panel::ViewerProps;

pub fn view<'a>(props: ViewerProps<'a>) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	viewer_panel::view(props).map(|message| PanelSystemMessage::AppMessage(AppPanelMessage::Preview(message)))
}
