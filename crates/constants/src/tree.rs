//! Outline rows shared by the Layers and Assets panels (Finder-style
//! sidebar rows: inset selection pill, disclosure chevron, tinted glyph).

use iced::widget::{Space, container, hover, mouse_area, row};
use iced::{Alignment, Border, Color, Element, Length};

use crate::icons::{Icon, icon, icon_transformed};
use crate::style::{
	BORDER_RADIUS, FONT_LABEL, FONT_TINY, ROW_HEIGHT, SPACE_2, SPACE_3, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR,
	TEXT_PRIMARY_COLOR_INVERTED, TREE_INDENT,
};
use crate::widgets::{list_row, mono_label, ui_label};

/// Disclosure state of a row that has children.
#[derive(Debug, Clone)]
pub struct Disclosure<Message> {
	pub expanded: bool,
	pub on_toggle: Message,
}

/// One outline row.
pub struct TreeRow<'a, Message> {
	pub depth: usize,
	pub disclosure: Option<Disclosure<Message>>,
	pub icon: Icon,
	pub icon_color: Color,
	pub label: &'a str,
	pub meta: Option<String>,
	pub selected: bool,
	pub on_select: Message,
	pub on_open: Message,
}

impl<'a, Message: Clone + 'a> TreeRow<'a, Message> {
	pub fn view(self) -> Element<'a, Message> {
		let selected = self.selected;
		let fg = |color: Color| if selected { TEXT_PRIMARY_COLOR_INVERTED } else { color };

		let disclosure: Element<'a, Message> = match self.disclosure {
			Some(Disclosure { expanded, on_toggle }) => mouse_area(
				container(icon_transformed(
					Icon::ChevronRight,
					9.0,
					fg(TEXT_MUTED_COLOR),
					if expanded { std::f32::consts::FRAC_PI_2 } else { 0.0 },
					1.0,
				))
				.width(14)
				.height(ROW_HEIGHT)
				.center_y(ROW_HEIGHT),
			)
			.on_press(on_toggle)
			.into(),
			None => Space::new().width(14).into(),
		};

		let mut content = row![
			Space::new().width(self.depth as f32 * TREE_INDENT),
			disclosure,
			icon(self.icon, 14.0, fg(self.icon_color)),
			ui_label(self.label, FONT_LABEL).color(fg(TEXT_PRIMARY_COLOR)),
			Space::new().width(Length::Fill),
		]
		.spacing(SPACE_3)
		.align_y(Alignment::Center)
		.height(ROW_HEIGHT);

		if let Some(meta) = self.meta {
			content = content.push(mono_label(meta, FONT_TINY).color(if selected {
				Color::from_rgba(1.0, 1.0, 1.0, 0.75)
			} else {
				TEXT_MUTED_COLOR
			}));
		}

		let base = container(content)
			.width(Length::Fill)
			.height(ROW_HEIGHT)
			.padding([0.0, SPACE_2 + 2.0])
			.style(list_row(selected));

		let row: Element<'a, Message> = if selected {
			base.into()
		} else {
			// ホバー時だけ淡いハイライトを重ねる
			hover(
				base,
				container(Space::new())
					.width(Length::Fill)
					.height(ROW_HEIGHT)
					.style(|_| container::Style {
						background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.05).into()),
						border: Border {
							radius: BORDER_RADIUS.into(),
							..Default::default()
						},
						..Default::default()
					}),
			)
		};

		container(mouse_area(row).on_press(self.on_select).on_double_click(self.on_open))
			.padding([0.0, SPACE_2 + 2.0])
			.into()
	}
}
