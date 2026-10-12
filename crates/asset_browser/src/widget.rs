use crate::{AssetBrowserMessage, AssetBrowserState};
use constants::icons::{Icon, icon};
use constants::style::*;
use constants::tree::{Disclosure, TreeRow};
use constants::widgets::{self, button_body_center, ui_label};
use domain::{Asset, AssetType, Project};
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text_input, tooltip};
use iced::{Alignment, Color, Element, Length};

#[derive(Debug)]
pub struct AssetBrowserWidget<'a> {
	project: &'a Project,
}

//* -------- Public API -------- */
impl<'a> AssetBrowserWidget<'a> {
	pub fn new(project: &'a Project) -> Self {
		Self { project }
	}

	pub fn view(self, state: &'a AssetBrowserState) -> Element<'a, AssetBrowserMessage> {
		let project = self.project;
		let filter = state.filter.trim().to_lowercase();

		let toolbar = row![
			text_input("Search", &state.filter)
				.on_input(AssetBrowserMessage::FilterChanged)
				.size(FONT_UI)
				.padding([2, 8])
				.width(Length::Fill)
				.style(widgets::search_input),
			tool_button(Icon::Import, "Import Media…", AssetBrowserMessage::ImportMedia),
			tool_button(Icon::FolderPlus, "New Folder", AssetBrowserMessage::NewFolder),
			tool_button(Icon::Insert, "Add to Timeline", AssetBrowserMessage::AddToTimeline),
		]
		.spacing(SPACE_1)
		.align_y(Alignment::Center)
		.padding([0.0, SPACE_4 + 2.0])
		.height(ROW_HEIGHT);

		let rows: Vec<Element<'a, AssetBrowserMessage>> = project
			.root_assets()
			.iter()
			.filter_map(|id| project.asset(*id))
			.filter(|asset| matches_filter(project, asset, &filter))
			.map(|asset| view_item(project, asset, state, &filter, 0))
			.collect();

		let body: Element<'a, AssetBrowserMessage> = if rows.is_empty() {
			empty_state(!filter.is_empty())
		} else {
			let clear_area = mouse_area(container(Space::new()).width(Length::Fill).height(Length::Fill))
				.on_press(AssetBrowserMessage::ClearSelection);
			scrollable(column![column(rows).spacing(1), clear_area].width(Length::Fill))
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

//* -------- Private API -------- */
fn tool_button<'a>(glyph: Icon, label: &'a str, message: AssetBrowserMessage) -> Element<'a, AssetBrowserMessage> {
	widgets::tip(
		button(button_body_center(icon(glyph, 13.0, TEXT_SECONDARY_COLOR)))
			.padding(0)
			.width(24)
			.height(ROW_HEIGHT)
			.style(widgets::button_ghost)
			.on_press(message),
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

fn kind_meta(kind: AssetType) -> &'static str {
	match kind {
		AssetType::Folder => "",
		AssetType::Composition => "COMP",
		AssetType::Image => "IMG",
		AssetType::Video => "VID",
		AssetType::Audio => "AUD",
	}
}

/// Name match, or any descendant matches (folders stay visible as context).
fn matches_filter(project: &Project, asset: &Asset, filter: &str) -> bool {
	filter.is_empty()
		|| asset.name.to_lowercase().contains(filter)
		|| asset
			.children
			.iter()
			.filter_map(|child| project.asset(*child))
			.any(|child| matches_filter(project, child, filter))
}

fn view_item<'a>(
	project: &'a Project,
	asset: &'a Asset,
	state: &'a AssetBrowserState,
	filter: &str,
	depth: usize,
) -> Element<'a, AssetBrowserMessage> {
	let searching = !filter.is_empty();
	// 検索中は一致を見せるためフォルダを開いた状態で表示
	let is_expanded = searching || state.expanded_ids.contains(&asset.id());
	let has_children = !asset.children.is_empty();
	let (glyph, color) = kind_glyph(asset.kind());
	let meta = kind_meta(asset.kind());

	let mut children = vec![
		TreeRow {
			depth,
			disclosure: has_children.then_some(Disclosure {
				expanded: is_expanded,
				on_toggle: AssetBrowserMessage::ToggleExpand(asset.id()),
			}),
			icon: glyph,
			icon_color: color,
			label: &asset.name,
			meta: (!meta.is_empty()).then(|| meta.to_string()),
			selected: state.selected_id == Some(asset.id()),
			on_select: AssetBrowserMessage::Select(asset.id()),
			on_open: AssetBrowserMessage::OpenItem(asset.id()),
		}
		.view(),
	];

	if is_expanded {
		for child in asset.children.iter().filter_map(|child| project.asset(*child)) {
			if matches_filter(project, child, filter) {
				children.push(view_item(project, child, state, filter, depth + 1));
			}
		}
	}

	column(children).spacing(1).into()
}
