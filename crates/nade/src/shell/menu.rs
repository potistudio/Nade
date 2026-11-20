//! Application menus: the menu bar in the title strip and right-click
//! context menus, drawn as floating panels over the window.

use iced::widget::{Space, button, column, container, mouse_area, row};
use iced::{Alignment, Element, Length, Padding, Point, Size};
use nade_constants::style::{FONT_UI, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR};
use nade_constants::widgets::{self, mono_label, ui_label};

use crate::message::{MenuMessage, Message};

/// Width of a dropdown / context menu.
pub const MENU_WIDTH: f32 = 240.0;
const ITEM_HEIGHT: f32 = nade_constants::style::ROW_HEIGHT;
const ITEM_SPACING: f32 = 1.0;
const SEPARATOR_HEIGHT: f32 = 9.0;
const PANEL_PADDING: f32 = 4.0;
/// Space kept free for the macOS traffic lights.
const BAR_INSET: f32 = if cfg!(target_os = "macos") { 78.0 } else { 8.0 };
const BAR_TITLE_PADDING: f32 = 10.0;

/// Which menu is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
	File,
	Edit,
	Composition,
	Layer,
	Effect,
	View,
	/// Right-click menu for whatever is selected.
	Context,
}

impl MenuKind {
	pub const BAR: [Self; 6] = [
		Self::File,
		Self::Edit,
		Self::Composition,
		Self::Layer,
		Self::Effect,
		Self::View,
	];

	pub fn title(self) -> &'static str {
		match self {
			Self::File => "File",
			Self::Edit => "Edit",
			Self::Composition => "Composition",
			Self::Layer => "Layer",
			Self::Effect => "Effect",
			Self::View => "View",
			Self::Context => "",
		}
	}
}

/// An open menu and where its top-left corner sits (window coordinates).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpenMenu {
	pub kind: MenuKind,
	pub position: Point,
}

/// A row of a menu.
pub enum Entry {
	Item {
		label: &'static str,
		shortcut: &'static str,
		/// `None` shows the item disabled.
		action: Option<Message>,
	},
	Separator,
}

impl Entry {
	pub fn item(label: &'static str, shortcut: &'static str, action: Message) -> Self {
		Self::Item {
			label,
			shortcut,
			action: Some(action),
		}
	}

	/// An item that is only enabled when `enabled` holds.
	pub fn when(enabled: bool, label: &'static str, shortcut: &'static str, action: Message) -> Self {
		Self::Item {
			label,
			shortcut,
			action: enabled.then_some(action),
		}
	}

	fn height(&self) -> f32 {
		match self {
			Self::Item { .. } => ITEM_HEIGHT,
			Self::Separator => SEPARATOR_HEIGHT,
		}
	}
}

/// Width of a menu bar title button.
fn title_width(kind: MenuKind) -> f32 {
	kind.title().chars().count() as f32 * 7.0 + BAR_TITLE_PADDING * 2.0
}

/// Left edge of each menu bar title (window coordinates).
pub fn bar_slots() -> impl Iterator<Item = (MenuKind, f32)> {
	MenuKind::BAR.into_iter().scan(BAR_INSET, |x, kind| {
		let left = *x;
		*x += title_width(kind);
		Some((kind, left))
	})
}

/// Where the dropdown of a menu bar title opens.
pub fn bar_menu_position(kind: MenuKind, bar_height: f32) -> Point {
	let x = bar_slots()
		.find(|(slot, _)| *slot == kind)
		.map_or(BAR_INSET, |(_, x)| x);
	Point::new(x, bar_height)
}

/// Menu bar titles for the title strip.
pub fn bar<'a>(open: Option<MenuKind>, height: f32) -> Element<'a, Message> {
	let titles = MenuKind::BAR.into_iter().map(|kind| {
		let active = open == Some(kind);
		let label = ui_label(kind.title(), FONT_UI).color(if active {
			TEXT_PRIMARY_COLOR
		} else {
			TEXT_SECONDARY_COLOR
		});
		let title = button(container(label).center_y(Length::Fill))
			.padding([0.0, BAR_TITLE_PADDING])
			.width(title_width(kind))
			.height(ITEM_HEIGHT)
			.style(widgets::button_menu_item(active))
			.on_press(Message::Menu(MenuMessage::ToggleBar(kind)));
		// メニューを開いたまま隣のタイトルへ移ると切り替わる（macOS と同じ）
		mouse_area(title)
			.on_enter(Message::Menu(MenuMessage::HoverBar(kind)))
			.into()
	});
	row![Space::new().width(BAR_INSET)]
		.extend(titles)
		.align_y(Alignment::Center)
		.height(height)
		.into()
}

/// Size of the floating panel for `entries`.
pub fn panel_size(entries: &[Entry]) -> Size {
	let rows: f32 = entries.iter().map(Entry::height).sum();
	let gaps = entries.len().saturating_sub(1) as f32 * ITEM_SPACING;
	Size::new(MENU_WIDTH, rows + gaps + PANEL_PADDING * 2.0 + 2.0)
}

/// Keep a menu of `size` inside the window.
pub fn clamp_position(position: Point, size: Size, window: Size) -> Point {
	Point::new(
		position.x.min(window.width - size.width - 4.0).max(4.0),
		position.y.min(window.height - size.height - 4.0).max(4.0),
	)
}

fn panel<'a>(entries: Vec<Entry>) -> Element<'a, Message> {
	let rows = entries.into_iter().map(|entry| -> Element<'a, Message> {
		match entry {
			Entry::Separator => container(widgets::separator())
				.height(SEPARATOR_HEIGHT)
				.center_y(SEPARATOR_HEIGHT)
				.padding([0.0, 6.0])
				.into(),
			Entry::Item {
				label,
				shortcut,
				action,
			} => {
				let enabled = action.is_some();
				let color = if enabled { TEXT_PRIMARY_COLOR } else { TEXT_MUTED_COLOR };
				let content = row![
					ui_label(label, FONT_UI).color(color),
					Space::new().width(Length::Fill),
					mono_label(shortcut, FONT_UI - 1.0).color(if enabled {
						TEXT_SECONDARY_COLOR
					} else {
						TEXT_MUTED_COLOR
					}),
				]
				.align_y(Alignment::Center)
				.width(Length::Fill);
				let mut item = button(widgets::button_body(content))
					.padding([0.0, 8.0])
					.width(Length::Fill)
					.height(ITEM_HEIGHT)
					.style(widgets::button_menu_item(false));
				if let Some(action) = action {
					item = item.on_press(Message::Menu(MenuMessage::Run(Box::new(action))));
				}
				item.into()
			}
		}
	});
	container(column(rows).spacing(ITEM_SPACING))
		.width(MENU_WIDTH)
		.padding(PANEL_PADDING)
		.style(widgets::editor_menu_panel)
		.into()
}

/// Full-window layer: a click outside closes the menu, the panel floats at
/// `position`.
pub fn overlay<'a>(entries: Vec<Entry>, position: Point) -> Element<'a, Message> {
	let dismiss = mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
		.on_press(Message::Menu(MenuMessage::Close))
		.on_right_press(Message::Menu(MenuMessage::Close));
	let placed = container(panel(entries)).padding(Padding {
		top: position.y,
		left: position.x,
		..Padding::ZERO
	});
	iced::widget::stack![dismiss, placed]
		.width(Length::Fill)
		.height(Length::Fill)
		.into()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn dropdowns_open_under_their_titles() {
		let slots: Vec<_> = bar_slots().collect();
		assert_eq!(slots[0], (MenuKind::File, BAR_INSET));
		assert!(slots.windows(2).all(|pair| pair[0].1 < pair[1].1));
		let edit = bar_menu_position(MenuKind::Edit, 28.0);
		assert_eq!(edit, Point::new(slots[1].1, 28.0));
	}

	#[test]
	fn menus_stay_inside_the_window() {
		let window = Size::new(800.0, 600.0);
		let size = Size::new(MENU_WIDTH, 200.0);
		let placed = clamp_position(Point::new(780.0, 590.0), size, window);
		assert_eq!(placed, Point::new(800.0 - MENU_WIDTH - 4.0, 396.0));
		assert_eq!(
			panel_size(&[Entry::Separator]).height,
			SEPARATOR_HEIGHT + PANEL_PADDING * 2.0 + 2.0
		);
	}
}
