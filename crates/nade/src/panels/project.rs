use std::collections::HashSet;
use uuid::Uuid;

use iced::widget::{column, container, mouse_area, row, text};
use iced::{Element, Length};

use crate::message::{AppPanelMessage, ProjectMessage};
use crate::panel_content::PanelContent;
use crate::theme::panel::COMPOSITION_BG;
use crate::theme::{TEXT_PRIMARY, TEXT_SECONDARY};
use panel_system::PanelSystemMessage;

// =============================================================================
// Data Models
// =============================================================================

#[derive(Debug, Clone)]
pub enum ItemType {
	Folder,
	Composition,
	Image,
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
// View Logic
// =============================================================================

pub fn view<'a>(
	data: &'a ProjectData,
	state: &'a ProjectUiState,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	let content = column(data.root_items.iter().map(|item| view_item(item, state, 0))).spacing(2);

	container(content)
		.width(Length::Fill)
		.height(Length::Fill)
		.padding(5)
		.style(|_| container::Style {
			background: Some(COMPOSITION_BG.into()),
			..Default::default()
		})
		.into()
}

fn view_item<'a>(
	item: &'a ProjectItem,
	state: &'a ProjectUiState,
	depth: usize,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
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
		ItemType::Audio => "🔊",
	};

	// Icon interaction: ToggleExpand for Folder/Composition, Select for others (or nothing)
	let icon = container(text(icon_str).size(14)).padding(2);
	let icon_element: Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> =
		match item.kind {
			ItemType::Folder | ItemType::Composition => mouse_area(icon)
				.on_press(PanelSystemMessage::AppMessage(AppPanelMessage::Project(
					ProjectMessage::ToggleExpand(item.id),
				)))
				.into(),
			_ => icon.into(),
		};

	// Label interaction: Select
	let label = text(&item.name).size(14).color(if is_selected {
		TEXT_PRIMARY
	} else {
		TEXT_SECONDARY
	});

	let label_area = mouse_area(
		container(label)
			.padding(2)
			.width(Length::Fill)
			.style(move |_| {
				if is_selected {
					container::Style {
						background: Some(iced::Color::from_rgb(0.2, 0.2, 0.3).into()),
						..Default::default()
					}
				} else {
					container::Style::default()
				}
			}),
	)
	.on_press(PanelSystemMessage::AppMessage(AppPanelMessage::Project(
		ProjectMessage::Select(item.id),
	)));

	let row_content = row![
		text(" ".repeat(depth * 3)).size(14), // Indentation
		icon_element,
		label_area
	]
	.spacing(5);

	let mut children = vec![row_content.into()];

	if is_expanded {
		for child in &item.children {
			children.push(view_item(child, state, depth + 1));
		}
	}

	column(children).into()
}
