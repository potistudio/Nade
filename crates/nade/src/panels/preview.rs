use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use crate::widgets::video_view::VideoView;
use constants::style::{FONT_UI, SPACE_1, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR};
use core::PreviewModel;
use iced::widget::{column, container, row, shader, text};
use iced::{Element, Length};
use panel_system::PanelSystemMessage;

/// プレビューパネルのビュー
pub fn view<'a>(preview: &PreviewModel) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	let content: Element<'_, PanelSystemMessage<PanelContent, AppPanelMessage>> = if let Some(frame) = &preview.frame {
		shader(VideoView::new(Some(frame.clone())))
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	} else {
		container(text("No Signal").size(FONT_UI).color(TEXT_MUTED_COLOR))
			.width(Length::Fill)
			.height(Length::Fill)
			.center_x(Length::Fill)
			.center_y(Length::Fill)
			.into()
	};

	let fps_text = text(format!("{:.1} FPS", preview.fps))
		.size(FONT_UI)
		.color(TEXT_SECONDARY_COLOR);

	let time_text = text(format!("Time: {:.2}s", preview.time))
		.size(FONT_UI)
		.color(TEXT_PRIMARY_COLOR);

	column![
		row![fps_text, text(" | ").size(FONT_UI).color(TEXT_MUTED_COLOR), time_text].spacing(SPACE_1),
		content,
	]
	.spacing(SPACE_1)
	.padding(SPACE_1)
	.into()
}
