use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use iced::Element;
use nade_panel_system::PanelSystemMessage;
use nade_viewer_panel::ViewerProps;

pub fn view<'a>(props: ViewerProps<'a>) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	nade_viewer_panel::view(props).map(|message| PanelSystemMessage::AppMessage(AppPanelMessage::Preview(message)))
}
