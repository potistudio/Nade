use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use iced::Element;
use panel_system::PanelSystemMessage;
use timeline_panel::{TimelineInteraction, TimelineModel, TimelineWidget};

pub fn view<'a>(
	panel_id: usize,
	state: &'a TimelineInteraction,
	model: &'a TimelineModel,
	current_time: f32,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	TimelineWidget::new(state, model)
		.view(current_time)
		.map(move |message| PanelSystemMessage::AppMessage(AppPanelMessage::Timeline { panel_id, message }))
}
