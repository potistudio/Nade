use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use iced::Element;
use panel_system::PanelSystemMessage;

use nade_core::Project;
use project_panel::{ProjectPaneState, ProjectPaneWidget};

pub fn view<'a>(
	project: &'a Project,
	state: &'a ProjectPaneState,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	ProjectPaneWidget::new(project)
		.view(state)
		.map(AppPanelMessage::Project)
		.map(PanelSystemMessage::AppMessage)
}
