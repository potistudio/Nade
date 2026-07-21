//! Flat Blender-like iced widget styles.
//!
//! Apply via `.style(widgets::button_tool)` etc. Theme alone only fixes the
//! palette; these Catalog style fns control borders, radius, and interaction.
//!
//! Also provides layout helpers: iced `button` places children at the **top-left**
//! of its padded region, which makes labels look vertically shifted. Always wrap
//! button content with [`button_body`] (or [`button_body_center`]) when using a
//! fixed height.

use iced::border::Radius;
use iced::widget::overlay::menu;
use iced::widget::text::LineHeight;
use iced::widget::{
	button, container, pick_list as pick_list_widget, row, scrollable as scrollable_widget, slider as slider_widget,
	text, text_input as text_input_widget,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Theme};

use crate::style::*;

fn widget_border(color: Color) -> Border {
	Border {
		color,
		width: BORDER_THICKNESS,
		radius: BORDER_RADIUS.into(),
	}
}

fn panel_border() -> Border {
	Border {
		color: BORDER_COLOR,
		width: BORDER_THICKNESS,
		radius: PANEL_RADIUS.into(),
	}
}

// ---------------------------------------------------------------------------
// Layout helpers (vertical alignment)
// ---------------------------------------------------------------------------

/// Vertically center content inside a `button` (width follows content / parent).
///
/// Do **not** force `Length::Fill` width here — that makes Shrink buttons (tool
/// chrome) expand across the row. For full-width menu rows, set `.width(Fill)`
/// on the `button` and give the inner content `width(Fill)` instead.
pub fn button_body<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
	container(content.into())
		.height(Length::Fill)
		.align_y(Alignment::Center)
		.into()
}

/// Vertically center content; horizontal centering when the button is wider than content.
pub fn button_body_center<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
	container(content.into())
		.height(Length::Fill)
		.align_x(Alignment::Center)
		.align_y(Alignment::Center)
		.into()
}

/// UI label with absolute line height so it optically matches icon boxes.
pub fn ui_label<'a>(label: impl text::IntoFragment<'a>, size: f32) -> text::Text<'a> {
	text(label).size(size).line_height(LineHeight::Absolute(size.into()))
}

/// Fixed square slot that centers an icon widget.
pub fn icon_slot<'a, Message: 'a>(icon: impl Into<Element<'a, Message>>, size: f32) -> Element<'a, Message> {
	container(icon.into()).width(size).height(size).center(size).into()
}

/// Icon + label row with shared vertical centering and tight text metrics.
pub fn icon_label_row<'a, Message: 'a>(
	icon: impl Into<Element<'a, Message>>,
	label: impl text::IntoFragment<'a>,
	label_size: f32,
	label_color: Color,
	icon_size: f32,
	spacing: f32,
) -> Element<'a, Message> {
	row![
		icon_slot(icon, icon_size),
		ui_label(label, label_size).color(label_color),
	]
	.spacing(spacing)
	.align_y(Alignment::Center)
	.width(Length::Fill)
	.into()
}

// ---------------------------------------------------------------------------
// Containers
// ---------------------------------------------------------------------------

pub fn window(_theme: &Theme) -> container::Style {
	container::Style {
		background: Some(BACKGROUND_COLOR.into()),
		text_color: Some(TEXT_PRIMARY_COLOR),
		..Default::default()
	}
}

pub fn panel(_theme: &Theme) -> container::Style {
	container::Style {
		background: Some(PANEL_COLOR.into()),
		text_color: Some(TEXT_PRIMARY_COLOR),
		border: panel_border(),
		..Default::default()
	}
}

pub fn panel_body(_theme: &Theme) -> container::Style {
	container::Style {
		background: Some(PANEL_COLOR.into()),
		text_color: Some(TEXT_PRIMARY_COLOR),
		border: Border {
			color: BORDER_COLOR,
			width: BORDER_THICKNESS,
			radius: PANEL_RADIUS.into(),
		},
		..Default::default()
	}
}

pub fn panel_header(_theme: &Theme) -> container::Style {
	container::Style {
		background: Some(HEADER_COLOR.into()),
		text_color: Some(TEXT_SECONDARY_COLOR),
		border: Border {
			color: BORDER_COLOR,
			width: 0.0,
			radius: PANEL_RADIUS.into(),
		},
		..Default::default()
	}
}

pub fn toolbar(_theme: &Theme) -> container::Style {
	container::Style {
		background: Some(BACKGROUND_COLOR.into()),
		text_color: Some(TEXT_SECONDARY_COLOR),
		border: Border {
			color: BORDER_COLOR,
			width: BORDER_THICKNESS,
			radius: PANEL_RADIUS.into(),
		},
		..Default::default()
	}
}

pub fn list_row(selected: bool) -> impl Fn(&Theme) -> container::Style {
	move |_theme| container::Style {
		background: Some(if selected {
			BACKGROUND_ACTIVE_COLOR.into()
		} else {
			PANEL_COLOR.into()
		}),
		text_color: Some(if selected {
			TEXT_PRIMARY_COLOR_INVERTED
		} else {
			TEXT_PRIMARY_COLOR
		}),
		..Default::default()
	}
}

// ---------------------------------------------------------------------------
// Buttons — flat tool / header chrome (no primary indigo, no shadow)
// ---------------------------------------------------------------------------

pub fn button_tool(_theme: &Theme, status: button::Status) -> button::Style {
	let (background, text_color, border_color) = match status {
		button::Status::Active => (Some(WIDGET_COLOR.into()), TEXT_SECONDARY_COLOR, BORDER_COLOR),
		button::Status::Hovered => (Some(WIDGET_HOVER_COLOR.into()), TEXT_PRIMARY_COLOR, BORDER_ACTIVE_COLOR),
		button::Status::Pressed => (
			Some(SELECTION_COLOR.into()),
			TEXT_PRIMARY_COLOR_INVERTED,
			SELECTION_COLOR,
		),
		button::Status::Disabled => (
			Some(Color { a: 0.5, ..WIDGET_COLOR }.into()),
			TEXT_MUTED_COLOR,
			BORDER_COLOR,
		),
	};

	button::Style {
		background,
		text_color,
		border: widget_border(border_color),
		shadow: Shadow::default(),
		snap: true,
	}
}

/// Transparent until hover — for header actions like Join.
pub fn button_ghost(_theme: &Theme, status: button::Status) -> button::Style {
	let (background, text_color) = match status {
		button::Status::Active => (None, TEXT_SECONDARY_COLOR),
		button::Status::Hovered => (Some(WIDGET_HOVER_COLOR.into()), TEXT_PRIMARY_COLOR),
		button::Status::Pressed => (Some(SELECTION_COLOR.into()), TEXT_PRIMARY_COLOR_INVERTED),
		button::Status::Disabled => (None, TEXT_MUTED_COLOR),
	};

	button::Style {
		background,
		text_color,
		border: Border {
			radius: BORDER_RADIUS.into(),
			..Default::default()
		},
		shadow: Shadow::default(),
		snap: true,
	}
}

/// Blender editor-type selector: compact dark chip with subtle border.
pub fn button_editor_type(_theme: &Theme, status: button::Status) -> button::Style {
	let (background, border_color) = match status {
		button::Status::Active => (Some(WIDGET_COLOR.into()), BORDER_SUBTLE_COLOR),
		button::Status::Hovered => (Some(WIDGET_HOVER_COLOR.into()), BORDER_ACTIVE_COLOR),
		button::Status::Pressed => (Some(BACKGROUND_SELECTED_COLOR.into()), BORDER_ACTIVE_COLOR),
		button::Status::Disabled => (Some(Color { a: 0.5, ..WIDGET_COLOR }.into()), BORDER_COLOR),
	};

	button::Style {
		background,
		text_color: TEXT_PRIMARY_COLOR,
		border: widget_border(border_color),
		shadow: Shadow::default(),
		snap: true,
	}
}

/// Blender editor-type menu row. `selected` draws the blue active highlight.
pub fn button_menu_item(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
	button_menu_item_faded(selected, 1.0)
}

/// Menu row with alpha for open/close animation.
pub fn button_menu_item_faded(selected: bool, alpha: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
	move |_theme, status| {
		let (background, text_color) = if selected {
			(
				Some(SELECTION_COLOR.scale_alpha(alpha).into()),
				TEXT_PRIMARY_COLOR_INVERTED.scale_alpha(alpha),
			)
		} else {
			match status {
				button::Status::Hovered => (
					Some(BACKGROUND_SELECTED_COLOR.scale_alpha(alpha).into()),
					TEXT_PRIMARY_COLOR.scale_alpha(alpha),
				),
				button::Status::Pressed => (
					Some(SELECTION_COLOR.scale_alpha(alpha).into()),
					TEXT_PRIMARY_COLOR_INVERTED.scale_alpha(alpha),
				),
				_ => (None, TEXT_PRIMARY_COLOR.scale_alpha(alpha)),
			}
		};

		button::Style {
			background,
			text_color,
			border: Border {
				radius: BORDER_RADIUS.into(),
				..Default::default()
			},
			shadow: Shadow::default(),
			snap: true,
		}
	}
}

pub fn editor_menu_panel(theme: &Theme) -> container::Style {
	editor_menu_panel_faded(theme, 1.0)
}

pub fn editor_menu_panel_faded(_theme: &Theme, alpha: f32) -> container::Style {
	container::Style {
		background: Some(WIDGET_COLOR.scale_alpha(alpha).into()),
		text_color: Some(TEXT_PRIMARY_COLOR.scale_alpha(alpha)),
		border: Border {
			color: BORDER_COLOR.scale_alpha(alpha),
			width: BORDER_THICKNESS,
			radius: BORDER_RADIUS.into(),
		},
		..Default::default()
	}
}

// ---------------------------------------------------------------------------
// Pick list + menu
// ---------------------------------------------------------------------------

pub fn pick_list(_theme: &Theme, status: pick_list_widget::Status) -> pick_list_widget::Style {
	let background = match status {
		pick_list_widget::Status::Active => WIDGET_COLOR,
		pick_list_widget::Status::Hovered | pick_list_widget::Status::Opened { .. } => WIDGET_HOVER_COLOR,
	};

	pick_list_widget::Style {
		text_color: TEXT_PRIMARY_COLOR,
		placeholder_color: TEXT_MUTED_COLOR,
		handle_color: TEXT_SECONDARY_COLOR,
		background: background.into(),
		border: widget_border(BORDER_COLOR),
	}
}

pub fn menu(_theme: &Theme) -> menu::Style {
	menu::Style {
		background: HEADER_COLOR.into(),
		border: Border {
			color: BORDER_COLOR,
			width: BORDER_THICKNESS,
			radius: PANEL_RADIUS.into(),
		},
		text_color: TEXT_PRIMARY_COLOR,
		selected_text_color: TEXT_PRIMARY_COLOR_INVERTED,
		selected_background: SELECTION_COLOR.into(),
		shadow: Shadow::default(),
	}
}

// ---------------------------------------------------------------------------
// Text input
// ---------------------------------------------------------------------------

pub fn text_input(_theme: &Theme, status: text_input_widget::Status) -> text_input_widget::Style {
	let border_color = match status {
		text_input_widget::Status::Active => BORDER_COLOR,
		text_input_widget::Status::Hovered => BORDER_ACTIVE_COLOR,
		text_input_widget::Status::Focused { .. } => SELECTION_COLOR,
		text_input_widget::Status::Disabled => BORDER_SUBTLE_COLOR,
	};

	text_input_widget::Style {
		background: Background::Color(WIDGET_COLOR),
		border: widget_border(border_color),
		icon: TEXT_SECONDARY_COLOR,
		placeholder: TEXT_MUTED_COLOR,
		value: TEXT_PRIMARY_COLOR,
		selection: SELECTION_COLOR,
	}
}

// ---------------------------------------------------------------------------
// Slider — thin rail, rectangular Blender-like handle
// ---------------------------------------------------------------------------

pub fn slider(_theme: &Theme, status: slider_widget::Status) -> slider_widget::Style {
	let handle_bg = match status {
		slider_widget::Status::Active => WIDGET_HOVER_COLOR,
		slider_widget::Status::Hovered | slider_widget::Status::Dragged => SELECTION_COLOR,
	};

	slider_widget::Style {
		rail: slider_widget::Rail {
			backgrounds: (SELECTION_COLOR.into(), WIDGET_COLOR.into()),
			width: 3.0,
			border: Border {
				radius: Radius::new(1.0),
				width: 0.0,
				color: Color::TRANSPARENT,
			},
		},
		handle: slider_widget::Handle {
			shape: slider_widget::HandleShape::Rectangle {
				width: 8,
				border_radius: BORDER_RADIUS.into(),
			},
			background: handle_bg.into(),
			border_width: BORDER_THICKNESS,
			border_color: BORDER_ACTIVE_COLOR,
		},
	}
}

// ---------------------------------------------------------------------------
// Scrollable — thin flat rails
// ---------------------------------------------------------------------------

pub fn scrollable(_theme: &Theme, status: scrollable_widget::Status) -> scrollable_widget::Style {
	let base_scroller = TEXT_MUTED_COLOR;
	let active_scroller = TEXT_SECONDARY_COLOR;

	let rail = |scroller_color: Color| scrollable_widget::Rail {
		background: Some(BACKGROUND_COLOR.into()),
		border: Border {
			radius: PANEL_RADIUS.into(),
			width: 0.0,
			color: Color::TRANSPARENT,
		},
		scroller: scrollable_widget::Scroller {
			background: scroller_color.into(),
			border: Border {
				radius: BORDER_RADIUS.into(),
				width: 0.0,
				color: Color::TRANSPARENT,
			},
		},
	};

	let base = rail(base_scroller);
	let hot = rail(active_scroller);

	let auto_scroll = scrollable_widget::AutoScroll {
		background: Color {
			a: 0.55,
			..BACKGROUND_COLOR
		}
		.into(),
		border: Border::default(),
		shadow: Shadow::default(),
		icon: TEXT_PRIMARY_COLOR,
	};

	match status {
		scrollable_widget::Status::Active { .. } => scrollable_widget::Style {
			container: container::Style::default(),
			vertical_rail: base,
			horizontal_rail: base,
			gap: None,
			auto_scroll,
		},
		scrollable_widget::Status::Hovered {
			is_horizontal_scrollbar_hovered,
			is_vertical_scrollbar_hovered,
			..
		} => scrollable_widget::Style {
			container: container::Style::default(),
			vertical_rail: if is_vertical_scrollbar_hovered { hot } else { base },
			horizontal_rail: if is_horizontal_scrollbar_hovered { hot } else { base },
			gap: None,
			auto_scroll,
		},
		scrollable_widget::Status::Dragged {
			is_horizontal_scrollbar_dragged,
			is_vertical_scrollbar_dragged,
			..
		} => scrollable_widget::Style {
			container: container::Style::default(),
			vertical_rail: if is_vertical_scrollbar_dragged { hot } else { base },
			horizontal_rail: if is_horizontal_scrollbar_dragged { hot } else { base },
			gap: None,
			auto_scroll,
		},
	}
}
