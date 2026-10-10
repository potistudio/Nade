use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use browser_panel::{ProjectPaneState, ProjectPaneWidget};
use domain::Project;
use iced::Element;
use panel_system::PanelSystemMessage;

pub fn view<'a>(
	project: &'a Project,
	state: &'a ProjectPaneState,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	ProjectPaneWidget::new(project)
		.view(state)
		.map(AppPanelMessage::Project)
		.map(PanelSystemMessage::AppMessage)
}
