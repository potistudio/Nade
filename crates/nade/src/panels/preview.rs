use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use crate::theme::TEXT_PRIMARY;
use crate::widgets::video_view::VideoView;
use core::PreviewModel;
use iced::widget::{column, container, row, shader, text};
use iced::{Color, Element, Length};
use panel_system::PanelSystemMessage;

/// プレビューパネルのビュー
pub fn view<'a>(
	preview: &PreviewModel,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	let content: Element<'_, PanelSystemMessage<PanelContent, AppPanelMessage>> =
		if let Some(frame) = &preview.frame {
			shader(VideoView::new(Some(frame.clone())))
				.width(Length::Fill)
				.height(Length::Fill)
				.into()
		} else {
			container(text("No Signal").color(TEXT_PRIMARY))
				.width(Length::Fill)
				.height(Length::Fill)
				.center_x(Length::Fill)
				.center_y(Length::Fill)
				.into()
		};

	let fps_text = text(format!("{:.1} FPS", preview.fps))
		.size(12)
		.color(Color::from_rgb(0.4, 0.8, 1.0));

	let time_text = text(format!("Time: {:.2}s", preview.time))
		.size(12)
		.color(Color::from_rgb(0.8, 0.8, 0.8));

	column![
		row![
			fps_text,
			text(" | ").size(12).color(TEXT_PRIMARY),
			time_text
		]
		.spacing(5),
		content,
	]
	.spacing(5)
	.padding(5)
	.into()
}
