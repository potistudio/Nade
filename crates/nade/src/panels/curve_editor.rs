use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use constants::icons::{Icon, icon};
use constants::style::{FONT_TITLE, FONT_UI, SPACE_3, TEXT_MUTED_COLOR, TEXT_SECONDARY_COLOR};
use constants::widgets::{self, ui_label};
use curve_editor_panel::{CurveEditorState, CurveEditorWidget};
use domain::TransformAnimation;
use iced::{
	Element, Length,
	widget::{column, container},
};
use panel_system::PanelSystemMessage;

pub fn view<'a>(
	state: &'a CurveEditorState,
	animation: Option<&'a TransformAnimation>,
	current_time: f32,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	let Some(animation) = animation else {
		return container(
			column![
				icon(Icon::PanelCurves, 28.0, TEXT_MUTED_COLOR),
				ui_label("No Layer Selected", FONT_TITLE).color(TEXT_SECONDARY_COLOR),
				ui_label("Select a layer to edit its animation curves.", FONT_UI).color(TEXT_MUTED_COLOR),
			]
			.spacing(SPACE_3 + 2.0)
			.align_x(iced::Alignment::Center),
		)
		.center(Length::Fill)
		.style(widgets::panel_surface)
		.into();
	};

	CurveEditorWidget::new(state, animation, current_time)
		.view()
		.map(|message| PanelSystemMessage::AppMessage(AppPanelMessage::CurveEditor(message)))
}
