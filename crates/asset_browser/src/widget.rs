use crate::{AssetBrowserMessage, AssetBrowserView, AssetRow};
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text_input, tooltip};
use iced::{Alignment, Color, Element, Length};
use nade_constants::icons::{Icon, icon};
use nade_constants::style::*;
use nade_constants::tree::{Disclosure, TreeRow};
use nade_constants::widgets::{self, button_body_center, ui_label};
use nade_domain::AssetType;
use std::borrow::Cow;

#[derive(Debug)]
pub struct AssetBrowserWidget<'a> {
	view: AssetBrowserView<'a>,
}

// -------- Public API ---------------------------------------------------------
impl<'a> AssetBrowserWidget<'a> {
	pub fn new(view: AssetBrowserView<'a>) -> Self {
		Self { view }
	}

	/// Renders the panel from the state-derived model.
	///
	/// Takes no `Project`: the tree was already flattened into rows, so the
	/// widget cannot depend on how assets are stored or looked up.
	pub fn view(self) -> Element<'a, AssetBrowserMessage> {
		let searching = self.view.is_searching();
		let AssetBrowserView {
			filter,
			rows,
			can_add_to_timeline,
		} = self.view;

		let toolbar = row![
			text_input("Search", filter)
				.on_input(AssetBrowserMessage::FilterChanged)
				.size(FONT_UI)
				.padding([2, 8])
				.width(Length::Fill)
				.style(widgets::search_input),
			tool_button(Icon::Import, "Import Media…", Some(AssetBrowserMessage::ImportMedia)),
			tool_button(Icon::FolderPlus, "New Folder", Some(AssetBrowserMessage::NewFolder)),
			tool_button(
				Icon::Insert,
				"Add to Timeline",
				can_add_to_timeline.then_some(AssetBrowserMessage::AddToTimeline),
			),
		]
		.spacing(SPACE_1)
		.align_y(Alignment::Center)
		.padding([0.0, SPACE_4 + 2.0])
		.height(ROW_HEIGHT);

		let body: Element<'a, AssetBrowserMessage> = if rows.is_empty() {
			empty_state(searching)
		} else {
			let clear_area = mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill))
				.on_press(AssetBrowserMessage::ClearSelection);
			scrollable(column![column(rows.into_iter().map(view_row)).spacing(1), clear_area].width(Length::Fill))
				.height(Length::Fill)
				.style(widgets::scrollable)
				.into()
		};

		container(column![toolbar, body])
			.width(Length::Fill)
			.height(Length::Fill)
			.style(widgets::panel_surface)
			.into()
	}
}

// -------- Internal -----------------------------------------------------------
fn tool_button<'a>(
	glyph: Icon,
	label: &'a str,
	message: Option<AssetBrowserMessage>,
) -> Element<'a, AssetBrowserMessage> {
	widgets::tip(
		button(button_body_center(icon(glyph, 13.0, TEXT_SECONDARY_COLOR)))
			.padding(0)
			.width(24)
			.height(ROW_HEIGHT)
			.style(widgets::button_ghost)
			.on_press_maybe(message),
		label,
		tooltip::Position::Bottom,
	)
}

fn empty_state<'a>(searching: bool) -> Element<'a, AssetBrowserMessage> {
	let (glyph, title, hint) = if searching {
		(Icon::ToolZoom, "No Matches", "Try a different name.")
	} else {
		(Icon::Import, "No Media", "Import footage, images or audio.")
	};
	container(
		column![
			icon(glyph, 24.0, TEXT_MUTED_COLOR),
			ui_label(title, FONT_UI).color(TEXT_SECONDARY_COLOR),
			ui_label(hint, FONT_TINY + 1.0).color(TEXT_MUTED_COLOR),
		]
		.spacing(SPACE_3)
		.align_x(Alignment::Center),
	)
	.center(Length::Fill)
	.into()
}

fn view_row<'a>(row: AssetRow<'a>) -> Element<'a, AssetBrowserMessage> {
	let (glyph, color) = kind_glyph(row.kind);
	TreeRow {
		depth: row.depth,
		disclosure: row.has_children.then_some(Disclosure {
			expanded: row.expanded,
			on_toggle: AssetBrowserMessage::ToggleExpand(row.id),
		}),
		icon: glyph,
		icon_color: color,
		label: row.label,
		meta: kind_meta(row.kind).map(Cow::Borrowed),
		selected: row.selected,
		on_select: AssetBrowserMessage::Select(row.id),
		on_open: AssetBrowserMessage::OpenItem(row.id),
	}
	.view()
}

/// Finder 風の種類別アイコンと色
fn kind_glyph(kind: AssetType) -> (Icon, Color) {
	match kind {
		AssetType::Folder => (Icon::Folder, ACCENT_BRIGHT_COLOR),
		AssetType::Composition => (Icon::Comp, ACCENT_BRIGHT_COLOR),
		AssetType::Image => (Icon::Image, SUCCESS_COLOR),
		AssetType::Video => (Icon::Video, LABEL_COLORS[1]),
		AssetType::Audio => (Icon::Audio, WARNING_COLOR),
	}
}

fn kind_meta(kind: AssetType) -> Option<&'static str> {
	match kind {
		AssetType::Folder => None,
		AssetType::Composition => Some("COMP"),
		AssetType::Image => Some("IMG"),
		AssetType::Video => Some("VID"),
		AssetType::Audio => Some("AUD"),
	}
}
