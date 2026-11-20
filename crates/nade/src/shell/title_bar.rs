//! Title strip: hosts the traffic lights (macOS), the document title and,
//! off macOS, the menu bar; dragging it moves the window.

use iced::widget::{container, mouse_area, row, stack};
use iced::{Alignment, Element, Length};
use nade_constants::style::{FONT_UI, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR};
use nade_constants::widgets::ui_label;

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
	// macOS はシステムメニューバーを使う。他の OS ではメニュータイトルをドラッグ領域の上に重ねる
	if cfg!(target_os = "macos") {
		return strip.into();
	}
	stack![strip, menu::bar(open_menu, TITLE_BAR_HEIGHT)]
		.width(Length::Fill)
		.height(TITLE_BAR_HEIGHT)
		.into()
}
