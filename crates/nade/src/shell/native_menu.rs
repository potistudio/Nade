//! macOS: the application menus live in the system menu bar.
//!
//! The items mirror [`MenuKind::BAR`] built by the app's `menu_entries`, so
//! the in-window bar (other platforms) and the native one stay in step.
//! Item ids are `"<menu>:<entry>"`; a click is resolved against freshly
//! built entries, so only the enabled state has to be kept in sync.

use std::cell::RefCell;

use iced::futures::Stream;
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};

use crate::message::{MenuMessage, Message};
use crate::shell::menu::{Entry, MenuKind};

const QUIT_ID: &str = "app.quit";

struct NativeMenu {
	/// Kept alive: dropping it would empty the menu bar.
	_menu: Menu,
	/// Per bar menu, per entry: the item (separators are `None`) and whether
	/// it is currently enabled.
	items: Vec<Vec<Option<(MenuItem, bool)>>>,
}

thread_local! {
	static MENU: RefCell<Option<NativeMenu>> = const { RefCell::new(None) };
}

/// Whether the menu bar has been installed.
pub fn is_installed() -> bool {
	MENU.with_borrow(Option::is_some)
}

/// Install the menu bar on first call, then mirror which items are enabled.
/// Must run on the main thread after the app finished launching.
pub fn sync(entries: impl Fn(MenuKind) -> Vec<Entry>) {
	MENU.with_borrow_mut(|slot| {
		let menu = slot.get_or_insert_with(|| build(&entries));
		for (kind, items) in MenuKind::BAR.into_iter().zip(&mut menu.items) {
			for (entry, item) in entries(kind).iter().zip(items.iter_mut()) {
				let (Entry::Item { action, .. }, Some((item, enabled))) = (entry, item) else {
					continue;
				};
				if *enabled != action.is_some() {
					*enabled = action.is_some();
					item.set_enabled(*enabled);
				}
			}
		}
	});
}

/// The message for a clicked item id, if it is still enabled.
pub fn action(id: &str, entries: impl Fn(MenuKind) -> Vec<Entry>) -> Option<Message> {
	if id == QUIT_ID {
		return Some(Message::CloseRequested(iced::window::Id::unique()));
	}
	let (menu, entry) = id.split_once(':')?;
	let kind = *MenuKind::BAR.get(menu.parse::<usize>().ok()?)?;
	match entries(kind).into_iter().nth(entry.parse().ok()?)? {
		Entry::Item { action, .. } => action,
		Entry::Separator => None,
	}
}

/// Menu clicks as app messages (for `Subscription::run`).
pub fn events() -> impl Stream<Item = Message> {
	let (sender, receiver) = iced::futures::channel::mpsc::unbounded();
	MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
		let _ = sender.unbounded_send(Message::Menu(MenuMessage::Native(event.id.0)));
	}));
	receiver
}

fn build(entries: &impl Fn(MenuKind) -> Vec<Entry>) -> NativeMenu {
	let menu = Menu::new();
	let quit = MenuItem::with_id(QUIT_ID, "Quit Nade", true, accelerator("⌘Q"));
	let app = Submenu::new("Nade", true);
	let about = PredefinedMenuItem::about(
		Some("About Nade"),
		Some(AboutMetadata {
			name: Some("Nade".into()),
			version: Some(env!("CARGO_PKG_VERSION").into()),
			..AboutMetadata::default()
		}),
	);
	for item in [
		&about as &dyn muda::IsMenuItem,
		&PredefinedMenuItem::separator(),
		&PredefinedMenuItem::services(None),
		&PredefinedMenuItem::separator(),
		&PredefinedMenuItem::hide(None),
		&PredefinedMenuItem::hide_others(None),
		&PredefinedMenuItem::show_all(None),
		&PredefinedMenuItem::separator(),
		&quit,
	] {
		append_to(&app, item);
	}
	append(&menu, &app);

	let mut items = Vec::new();
	for (index, kind) in MenuKind::BAR.into_iter().enumerate() {
		let submenu = Submenu::new(kind.title(), true);
		let mut row = Vec::new();
		for (entry_index, entry) in entries(kind).into_iter().enumerate() {
			match entry {
				Entry::Separator => {
					append_to(&submenu, &PredefinedMenuItem::separator());
					row.push(None);
				}
				Entry::Item {
					label,
					shortcut,
					action,
				} => {
					let enabled = action.is_some();
					let item =
						MenuItem::with_id(format!("{index}:{entry_index}"), label, enabled, accelerator(shortcut));
					append_to(&submenu, &item);
					row.push(Some((item, enabled)));
				}
			}
		}
		append(&menu, &submenu);
		items.push(row);
	}

	let window = Submenu::new("Window", true);
	append_to(&window, &PredefinedMenuItem::fullscreen(None));
	append_to(&window, &PredefinedMenuItem::maximize(None));
	append_to(&window, &PredefinedMenuItem::separator());
	append_to(&window, &PredefinedMenuItem::bring_all_to_front(None));
	append(&menu, &window);
	window.set_as_windows_menu_for_nsapp();

	menu.init_for_nsapp();
	log::info!("NativeMenu: installed the macOS menu bar");
	NativeMenu { _menu: menu, items }
}

fn append(menu: &Menu, item: &dyn muda::IsMenuItem) {
	if let Err(error) = menu.append(item) {
		log::error!("NativeMenu: failed to add a menu: {error}");
	}
}

fn append_to(submenu: &Submenu, item: &dyn muda::IsMenuItem) {
	if let Err(error) = submenu.append(item) {
		log::error!("NativeMenu: failed to add a menu item: {error}");
	}
}

/// Key equivalent for a `⇧⌘S`-style shortcut label.
///
/// Only ⌘ shortcuts get one: a bare key (`B`, `[`, `F9`) would be swallowed
/// by the menu even while typing. ⌘X / ⌘C / ⌘V stay with the focused text
/// field, which receives them as normal key presses (the app's own shortcut
/// handler still copies layers when no field has focus).
fn accelerator(shortcut: &str) -> Option<Accelerator> {
	let mut modifiers = Modifiers::empty();
	let mut key = None;
	for symbol in shortcut.chars() {
		match symbol {
			'⌘' => modifiers |= Modifiers::META,
			'⇧' => modifiers |= Modifiers::SHIFT,
			'⌥' => modifiers |= Modifiers::ALT,
			'⌃' => modifiers |= Modifiers::CONTROL,
			other => key = Some(other.to_ascii_uppercase()),
		}
	}
	let key = key.filter(char::is_ascii_alphanumeric)?;
	if !modifiers.contains(Modifiers::META) || (modifiers == Modifiers::META && matches!(key, 'X' | 'C' | 'V')) {
		return None;
	}
	let code = if key.is_ascii_digit() {
		format!("Digit{key}")
	} else {
		format!("Key{key}")
	};
	Some(Accelerator::new(modifiers, code.parse::<Code>().ok()?))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn only_command_shortcuts_become_key_equivalents() {
		let save_as = accelerator("⇧⌘S").expect("⇧⌘S");
		assert_eq!(save_as.modifiers(), Modifiers::META | Modifiers::SHIFT);
		assert_eq!(save_as.key(), Code::KeyS);
		assert!(accelerator("⇧⌘C").is_some(), "pre-compose");
		for label in ["", "B", "[", "⌥[", "F9", "⌫", "Home", "⌘C", "⌘X", "⌘V"] {
			assert!(accelerator(label).is_none(), "{label}");
		}
	}

	#[test]
	fn ids_resolve_to_entry_actions() {
		let entries = |kind: MenuKind| match kind {
			MenuKind::Edit => vec![
				Entry::item("Undo", "⌘Z", Message::Tick),
				Entry::Separator,
				Entry::Item {
					label: "Copy",
					shortcut: "⌘C",
					action: None,
				},
			],
			_ => Vec::new(),
		};
		assert!(matches!(action("1:0", entries), Some(Message::Tick)));
		assert!(action("1:1", entries).is_none(), "separator");
		assert!(action("1:2", entries).is_none(), "disabled");
		assert!(action("9:0", entries).is_none());
		assert!(matches!(action(QUIT_ID, entries), Some(Message::CloseRequested(_))));
	}
}
