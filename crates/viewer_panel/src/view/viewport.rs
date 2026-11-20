//! The picture with its interactive overlay.

use constants::style::{FONT_UI, TEXT_MUTED_COLOR};
use constants::widgets::{self, ui_label};
use iced::widget::{Canvas, container, shader, stack};
use iced::{Element, Length, Size, Vector};

use crate::message::PreviewMessage;
use crate::view::ViewerProps;
use crate::widgets::video_view::VideoView;
use crate::widgets::viewer_overlay::ViewerOverlay;

pub(super) fn viewport<'a>(props: &ViewerProps<'a>) -> Element<'a, PreviewMessage> {
	let frame_size = props.composition.as_ref().map_or(Size::new(1920.0, 1080.0), |comp| {
		Size::new(comp.width as f32, comp.height as f32)
	});

	let overlay = Canvas::new(ViewerOverlay {
		frame: frame_size,
		zoom: props.preview.view_zoom,
		offset: Vector::new(props.preview.view_offset[0], props.preview.view_offset[1]),
		tool: props.tool,
		layers: props.layers.clone(),
		selected: props.selected,
		show_grid: props.settings.show_grid,
		show_safe_areas: props.settings.show_safe_areas,
	})
	.width(Length::Fill)
	.height(Length::Fill);

	let picture: Element<'a, PreviewMessage> = match &props.preview.frame {
		Some(frame) => shader(VideoView::new(
			Some(frame.clone()),
			props.preview.view_zoom,
			props.preview.view_offset,
		))
		.width(Length::Fill)
		.height(Length::Fill)
		.into(),
		None => container(ui_label("No Signal", FONT_UI).color(TEXT_MUTED_COLOR))
			.center(Length::Fill)
			.into(),
	};

	container(stack![picture, overlay].width(Length::Fill).height(Length::Fill))
		.width(Length::Fill)
		.height(Length::Fill)
		.style(widgets::well)
		.into()
}
