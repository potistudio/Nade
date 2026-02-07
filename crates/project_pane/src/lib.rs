//! # Project Pane
//!
//! Icedアプリケーション用のプロジェクトパネル。

use std::collections::HashSet;

use iced::widget::{column, container, mouse_area, row, text};
use iced::{Color, Element, Length};
use uuid::Uuid;

// =============================================================================
// Messages
// =============================================================================

/// プロジェクトパネルメッセージ
#[derive(Debug, Clone)]
pub enum ProjectMessage {
	ToggleExpand(Uuid),
	Select(Uuid),
	OpenItem(Uuid),
}

// =============================================================================
// Data Models
// =============================================================================

#[derive(Debug, Clone)]
pub enum ItemType {
	Folder,
	Composition,
	Image,
	Video,
	Audio,
}

#[derive(Debug, Clone)]
pub struct ProjectItem {
	pub id: Uuid,
	pub name: String,
	pub kind: ItemType,
	pub children: Vec<ProjectItem>,
}

impl ProjectItem {
	pub fn new(name: &str, kind: ItemType) -> Self {
		Self {
			id: Uuid::new_v4(),
			name: name.to_string(),
			kind,
			children: Vec::new(),
		}
	}

	pub fn with_child(mut self, child: ProjectItem) -> Self {
		self.children.push(child);
		self
	}
}

#[derive(Debug, Clone)]
pub struct ProjectData {
	pub root_items: Vec<ProjectItem>,
}

impl Default for ProjectData {
	fn default() -> Self {
		// Mock Data
		Self {
			root_items: vec![
				ProjectItem::new("Assets", ItemType::Folder)
					.with_child(ProjectItem::new("Background.png", ItemType::Image))
					.with_child(ProjectItem::new("Character.png", ItemType::Image))
					.with_child(ProjectItem::new("BGM.mp3", ItemType::Audio)),
				ProjectItem::new("Scenes", ItemType::Folder)
					.with_child(ProjectItem::new("Scene 1", ItemType::Composition))
					.with_child(ProjectItem::new("Scene 2", ItemType::Composition)),
				ProjectItem::new("Main Composition", ItemType::Composition),
			],
		}
	}
}

// =============================================================================
// UI State
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct ProjectUiState {
	pub expanded_ids: HashSet<Uuid>,
	pub selected_id: Option<Uuid>,
}

// =============================================================================
// Style
// =============================================================================

#[derive(Debug, Clone, Copy)]
pub struct ProjectPaneStyle {
	pub background: Color,
	pub text_primary: Color,
	pub text_secondary: Color,
	pub selected_background: Color,
}

// =============================================================================
// View Logic
// =============================================================================

#[derive(Debug, Clone, Copy)]
pub struct ProjectPane {
	style: ProjectPaneStyle,
}

impl ProjectPane {
	pub fn new(style: ProjectPaneStyle) -> Self {
		Self { style }
	}

	pub fn view<'a>(
		&self,
		data: &'a ProjectData,
		state: &'a ProjectUiState,
	) -> Element<'a, ProjectMessage> {
		let style = self.style;
		let content = column(
			data.root_items
				.iter()
				.map(|item| view_item(item, state, 0, style)),
		)
		.spacing(2);

		container(content)
			.width(Length::Fill)
			.height(Length::Fill)
			.padding(5)
			.style(move |_| container::Style {
				background: Some(style.background.into()),
				..Default::default()
			})
			.into()
	}
}

fn view_item<'a>(
	item: &'a ProjectItem,
	state: &'a ProjectUiState,
	depth: usize,
	style: ProjectPaneStyle,
) -> Element<'a, ProjectMessage> {
	let is_expanded = state.expanded_ids.contains(&item.id);
	let is_selected = state.selected_id == Some(item.id);

	let icon_str = match item.kind {
		ItemType::Folder => {
			if is_expanded {
				"📂"
			} else {
				"📁"
			}
		}
		ItemType::Composition => "🎬",
		ItemType::Image => "🖼️",
		ItemType::Video => "🎞️",
		ItemType::Audio => "🔊",
	};

	// Icon interaction: ToggleExpand for Folder/Composition, Select for others (or nothing)
	let icon = container(text(icon_str).size(14)).padding(2);
	let icon_element: Element<'a, ProjectMessage> = match item.kind {
		ItemType::Folder | ItemType::Composition => mouse_area(icon)
			.on_press(ProjectMessage::ToggleExpand(item.id))
			.into(),
		_ => icon.into(),
	};

	// Label interaction: Select
	let label = text(&item.name).size(14).color(if is_selected {
		style.text_primary
	} else {
		style.text_secondary
	});

	let label_area = mouse_area(
		container(label)
			.padding(2)
			.width(Length::Fill)
			.style(move |_| {
				if is_selected {
					container::Style {
						background: Some(style.selected_background.into()),
						..Default::default()
					}
				} else {
					container::Style::default()
				}
			}),
	)
	.on_press(ProjectMessage::Select(item.id));

	let row_content = row![
		text(" ".repeat(depth * 3)).size(14),
		icon_element,
		label_area
	]
	.spacing(5);

	let mut children = vec![row_content.into()];

	if is_expanded {
		for child in &item.children {
			children.push(view_item(child, state, depth + 1, style));
		}
	}

	column(children).into()
}
