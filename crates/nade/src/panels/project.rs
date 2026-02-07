use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use crate::theme;
use iced::{Color, Element};
use panel_system::PanelSystemMessage;
use project_pane::{ProjectPane, ProjectPaneStyle};

pub use project_pane::{ProjectData, ProjectUiState};

pub fn view<'a>(
	data: &'a ProjectData,
	state: &'a ProjectUiState,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	let style = ProjectPaneStyle {
		background: theme::panel::COMPOSITION_BG,
		text_primary: theme::TEXT_PRIMARY,
		text_secondary: theme::TEXT_SECONDARY,
		selected_background: Color::from_rgba(0.25, 0.35, 0.55, 0.35),
	};

	ProjectPane::new(style)
		.view(data, state)
		.map(AppPanelMessage::Project)
		.map(PanelSystemMessage::AppMessage)
}
