use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use curve_editor_panel::{CurveEditorState, CurveEditorWidget};
use domain::TransformAnimation;
use iced::{
	Element, Length,
	widget::{container, text},
};
use panel_system::PanelSystemMessage;

pub fn view<'a>(
	panel_id: usize,
	state: &'a CurveEditorState,
	animation: Option<&'a TransformAnimation>,
	current_time: f32,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	let Some(animation) = animation else {
		return container(text("Select an instance to edit animation curves.").size(12))
			.center(Length::Fill)
			.into();
	};

	CurveEditorWidget::new(state, animation, current_time)
		.view()
		.map(move |message| PanelSystemMessage::AppMessage(AppPanelMessage::CurveEditor { panel_id, message }))
}
