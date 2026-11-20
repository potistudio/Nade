use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use iced::Element;
use nade_asset_browser::{AssetBrowserState, AssetBrowserWidget};
use nade_domain::Project;
use nade_panel_system::PanelSystemMessage;

pub fn view<'a>(
	project: &'a Project,
	state: &'a AssetBrowserState,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	AssetBrowserWidget::new(project)
		.view(state)
		.map(AppPanelMessage::Assets)
		.map(PanelSystemMessage::AppMessage)
}
