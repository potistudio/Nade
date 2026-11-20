use std::collections::HashMap;

use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use iced::Element;
use nade_panel_system::PanelSystemMessage;
use nade_timeline_panel::{DropPreview, TimelineInteraction, TimelineModel, TimelineWidget};

pub struct TimelineProps<'a> {
	pub state: &'a TimelineInteraction,
	pub model: &'a TimelineModel,
	pub current_time: f32,
	pub fps: f32,
	pub duration: f32,
	/// Keyframe times per clip id.
	pub keyframes: HashMap<usize, Vec<f32>>,
	pub work_area: [f32; 2],
	/// Ghost of an asset being dragged in from the asset browser.
	pub drop_preview: Option<DropPreview>,
}

pub fn view<'a>(props: TimelineProps<'a>) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	TimelineWidget::new(props.state, props.model)
		.frame_rate(props.fps)
		.duration(props.duration)
		.keyframes(props.keyframes)
		.work_area(props.work_area)
		.drop_preview(props.drop_preview)
		.view(props.current_time)
		.map(|message| PanelSystemMessage::AppMessage(AppPanelMessage::Timeline(message)))
}
