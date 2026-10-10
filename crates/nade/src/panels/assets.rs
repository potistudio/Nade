use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use asset_browser::{AssetBrowserState, AssetBrowserWidget};
use domain::Project;
use iced::Element;
use panel_system::PanelSystemMessage;

pub fn view<'a>(
	project: &'a Project,
	state: &'a AssetBrowserState,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	AssetBrowserWidget::new(project)
		.view(state)
		.map(AppPanelMessage::Assets)
		.map(PanelSystemMessage::AppMessage)
}
