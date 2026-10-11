use std::collections::HashMap;

use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use iced::Element;
use panel_system::PanelSystemMessage;
use timeline_panel::{TimelineInteraction, TimelineModel, TimelineWidget};

pub struct TimelineProps<'a> {
	pub state: &'a TimelineInteraction,
	pub model: &'a TimelineModel,
	pub current_time: f32,
	pub fps: f32,
	pub duration: f32,
	/// Keyframe times per clip id.
	pub keyframes: HashMap<usize, Vec<f32>>,
}

pub fn view<'a>(
	panel_id: usize,
	props: TimelineProps<'a>,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	TimelineWidget::new(props.state, props.model)
		.frame_rate(props.fps)
		.duration(props.duration)
		.keyframes(props.keyframes)
		.view(props.current_time)
		.map(move |message| PanelSystemMessage::AppMessage(AppPanelMessage::Timeline { panel_id, message }))
}
