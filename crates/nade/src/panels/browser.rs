use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use iced::Element;
use nade_browser_panel::{ProjectPaneState, ProjectPaneWidget};
use nade_domain::Project;
use nade_panel_system::PanelSystemMessage;

pub fn view<'a>(
	project: &'a Project,
	state: &'a ProjectPaneState,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	ProjectPaneWidget::new(project)
		.view(state)
		.map(AppPanelMessage::Project)
		.map(PanelSystemMessage::AppMessage)
}
