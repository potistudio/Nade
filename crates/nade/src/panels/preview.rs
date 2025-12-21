use crate::message::{AppPanelMessage, Message};
use crate::panel_content::PanelContent;
use crate::theme::TEXT_PRIMARY;
use iced::widget::{Image, column, container, image, row, text};
use iced::{Color, Element, Length};
use nade_core::PreviewModel;
use panel_system::PanelSystemMessage;

/// プレビューパネルのビュー
pub fn view<'a>(
	preview: &PreviewModel,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	let content = if let Some(frame) = &preview.frame {
		// bytes::Bytes を使用してコピーを回避
		let handle = image::Handle::from_rgba(frame.width, frame.height, frame.pixels.clone());

		let img = Image::new(handle)
			.content_fit(iced::ContentFit::Contain)
			.width(Length::Fill)
			.height(Length::Fill);

		container(img).width(Length::Fill).height(Length::Fill)
	} else {
		container(text("No Signal").color(TEXT_PRIMARY))
			.width(Length::Fill)
			.height(Length::Fill)
			.center_x(Length::Fill)
			.center_y(Length::Fill)
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
