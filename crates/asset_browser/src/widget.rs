use crate::{
	AssetBrowserMessage, AssetBrowserView, AssetRow, DropTarget, ImportingRow, OutlineRow, SortDirection, SortKey,
	SortOrder,
};
use iced::widget::text::LineHeight;
use iced::widget::{Space, button, column, container, mouse_area, opaque, row, scrollable, stack, text_input, tooltip};
use iced::{Alignment, Border, Color, Element, Length};
use nade_constants::icons::{Icon, icon, icon_transformed};
use nade_constants::style::*;
use nade_constants::tree::{Disclosure, TreeRow};
use nade_constants::widgets::{self, button_body, button_body_center, mono_label, ui_label};
use nade_domain::AssetType;
use std::borrow::Cow;

#[derive(Debug)]
pub struct AssetBrowserWidget<'a> {
	view: AssetBrowserView<'a>,
	spinner_angle: f32,
}

// -------- Public API ---------------------------------------------------------
impl<'a> AssetBrowserWidget<'a> {
	pub fn new(view: AssetBrowserView<'a>) -> Self {
		Self {
			view,
			spinner_angle: 0.0,
		}
	}

	/// Rotation (radians) of the spinners on importing rows; the caller
	/// advances it while imports are pending.
	pub fn spinner_angle(mut self, angle: f32) -> Self {
		self.spinner_angle = angle;
		self
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
			drop_target,
			sort,
			can_add_to_timeline,
		} = self.view;
		let spinner_angle = self.spinner_angle;

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
			let rows = rows.into_iter().map(|row| match row {
				OutlineRow::Asset(row) => view_row(row),
				OutlineRow::Importing(row) => importing_row(row, spinner_angle),
			});
			let list = scrollable(column(rows).spacing(1).width(Length::Fill))
				.height(Length::Fill)
				.style(widgets::scrollable);
			// 縦スクロールの中では Fill が 0 に潰れるので、一覧ごと包んで空き領域の押下で解除する。
			// 行・シェブロン・スクロールバーの押下はそれぞれが捕捉するのでここまで届かない。
			let list = mouse_area(list).on_press(AssetBrowserMessage::ClearSelection);
			// ヘッダーはスクロールさせず、一覧の上に固定する
			column![sort_header(sort), widgets::separator(), list].into()
		};

		let panel = container(column![toolbar, body])
			.width(Length::Fill)
			.height(Length::Fill)
			.style(widgets::panel_surface);

		match drop_target {
			Some(target) => stack![panel, drop_overlay(target)].into(),
			None => panel.into(),
		}
	}
}

/// Widget ID of the in-place rename field, for focusing it once it appears.
pub fn rename_input_id() -> iced::widget::Id {
	iced::widget::Id::new("asset-browser-rename")
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

/// Finder-style column headers: click to sort, click again to flip.
///
/// Insets line the labels up with the row content below (label on the left,
/// kind tag on the right).
fn sort_header<'a>(sort: SortOrder) -> Element<'a, AssetBrowserMessage> {
	row![
		header_button("Name", SortKey::Name, sort).width(Length::Fill),
		header_button("Kind", SortKey::Kind, sort),
	]
	.spacing(SPACE_1)
	.padding([SPACE_1, SPACE_4])
	.height(ROW_HEIGHT)
	.into()
}

fn header_button<'a>(label: &'a str, key: SortKey, sort: SortOrder) -> iced::widget::Button<'a, AssetBrowserMessage> {
	let active = sort.key == key;
	let mut content =
		row![ui_label(label, FONT_TINY + 1.0).color(if active { TEXT_SECONDARY_COLOR } else { TEXT_MUTED_COLOR })]
			.spacing(SPACE_2)
			.align_y(Alignment::Center);
	if active {
		// 昇順は上向き、降順は下向きのシェブロン
		let rotation = match sort.direction {
			SortDirection::Ascending => std::f32::consts::PI,
			SortDirection::Descending => 0.0,
		};
		content = content.push(icon_transformed(
			Icon::ChevronDown,
			8.0,
			TEXT_SECONDARY_COLOR,
			rotation,
			1.0,
		));
	}

	button(button_body(content))
		.padding([0.0, SPACE_2])
		.height(Length::Fill)
		.style(widgets::button_ghost)
		.on_press(AssetBrowserMessage::SortBy(key))
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
		editor: row.rename.map(|draft| rename_field(row.label, draft)),
		meta: kind_meta(row.kind).map(Cow::Borrowed),
		selected: row.selected,
		on_select: AssetBrowserMessage::Select(row.id),
		on_open: AssetBrowserMessage::OpenItem(row.id),
	}
	.view()
}

/// Placeholder row for a file being probed: same metrics as [`TreeRow`],
/// muted, with a spinner in the icon slot. Not interactive.
fn importing_row<'a>(row: ImportingRow<'a>, spinner_angle: f32) -> Element<'a, AssetBrowserMessage> {
	let content = row![
		Space::new().width(row.depth as f32 * TREE_INDENT),
		// 開閉シェブロンの枠
		Space::new().width(14),
		icon_transformed(Icon::Spinner, 14.0, kind_glyph(row.kind).1, spinner_angle, 1.0),
		ui_label(row.label, FONT_LABEL).color(TEXT_SECONDARY_COLOR),
		Space::new().width(Length::Fill),
		mono_label("IMPORTING", FONT_TINY).color(TEXT_MUTED_COLOR),
	]
	.spacing(SPACE_3)
	.align_y(Alignment::Center)
	.height(ROW_HEIGHT);

	// 押しても一覧の「空き領域クリックで選択解除」に抜けないよう、イベントを止める
	opaque(
		container(
			container(content)
				.width(Length::Fill)
				.height(ROW_HEIGHT)
				.padding([0.0, SPACE_2 + 2.0]),
		)
		.padding([0.0, SPACE_2 + 2.0]),
	)
}

/// Accent frame over the whole panel while files are dragged over the window.
fn drop_overlay<'a>(target: DropTarget<'a>) -> Element<'a, AssetBrowserMessage> {
	let destination = match target.folder {
		Some(folder) => format!("Into “{folder}”"),
		None => "Into Project".to_owned(),
	};
	// 一覧の文字と重ならないよう、案内は不透明なカードに載せる
	let card = container(
		column![
			icon(Icon::Import, 24.0, ACCENT_BRIGHT_COLOR),
			ui_label("Drop to Import", FONT_UI).color(TEXT_PRIMARY_COLOR),
			ui_label(destination, FONT_TINY + 1.0).color(TEXT_SECONDARY_COLOR),
		]
		.spacing(SPACE_3)
		.align_x(Alignment::Center),
	)
	.padding([SPACE_5, SPACE_6])
	.style(widgets::hud(1.0));
	container(card)
		.center(Length::Fill)
		.style(|_| container::Style {
			background: Some(
				Color {
					a: 0.12,
					..ACCENT_COLOR
				}
				.into(),
			),
			border: Border {
				color: ACCENT_COLOR,
				width: 1.5,
				radius: BORDER_RADIUS.into(),
			},
			..Default::default()
		})
		.into()
}

/// In-place name field: Enter commits, Esc (handled by the shell) cancels.
fn rename_field<'a>(placeholder: &'a str, draft: &'a str) -> Element<'a, AssetBrowserMessage> {
	text_input(placeholder, draft)
		.id(rename_input_id())
		.on_input(AssetBrowserMessage::RenameInput)
		.on_submit(AssetBrowserMessage::CommitRename)
		.size(FONT_LABEL)
		// 16px の行送り + 上下 2px で、行の中身の高さ 20px に揃える
		.line_height(LineHeight::Absolute(16.0.into()))
		.padding([2, 4])
		.width(Length::Fill)
		.style(widgets::text_input)
		.into()
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
