//! Transparent title strip (macOS). Hosts the traffic lights and the
//! document title; dragging it moves the window.

use constants::style::{FONT_UI, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR};
use constants::widgets::ui_label;
use iced::widget::{container, mouse_area, row, stack};
use iced::{Alignment, Element, Length};

use crate::message::{Message, ShellMessage};
use crate::shell::menu::{self, MenuKind};
use crate::theme::TITLE_BAR_HEIGHT;

pub fn view<'a>(
	project_name: &'a str,
	composition_name: Option<&'a str>,
	open_menu: Option<MenuKind>,
) -> Element<'a, Message> {
	let mut title = row![ui_label(project_name, FONT_UI).color(TEXT_PRIMARY_COLOR)]
		.spacing(6)
		.align_y(Alignment::Center);
	if let Some(name) = composition_name {
		title = title
			.push(ui_label("—", FONT_UI).color(TEXT_MUTED_COLOR))
			.push(ui_label(name, FONT_UI).color(TEXT_SECONDARY_COLOR));
	}

	let strip = mouse_area(
		container(title)
			.width(Length::Fill)
			.height(TITLE_BAR_HEIGHT)
			.center_x(Length::Fill)
			.center_y(TITLE_BAR_HEIGHT),
	)
	.on_press(Message::Shell(ShellMessage::DragWindow))
	.on_double_click(Message::Shell(ShellMessage::ToggleMaximize));
	// メニュータイトルはドラッグ領域の上に重ねる
	stack![strip, menu::bar(open_menu, TITLE_BAR_HEIGHT)]
		.width(Length::Fill)
		.height(TITLE_BAR_HEIGHT)
		.into()
}
