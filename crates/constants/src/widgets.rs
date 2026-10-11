//! Graphite iced widget styles.
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
	button, container, pick_list as pick_list_widget, row, rule, scrollable as scrollable_widget,
	slider as slider_widget, text, text_input as text_input_widget, tooltip as tooltip_widget,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Theme, Vector};

use crate::style::*;

/// Translucent white used for hover / pressed overlays on any surface.
const fn white(alpha: f32) -> Color {
	Color::from_rgba(1.0, 1.0, 1.0, alpha)
}

fn rounded(radius: f32) -> Border {
	Border {
		radius: radius.into(),
		..Default::default()
	}
}

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

/// Soft drop shadow for floating surfaces (menus, HUDs, tooltips).
pub fn float_shadow(alpha: f32) -> Shadow {
	Shadow {
		color: Color::from_rgba(0.0, 0.0, 0.0, 0.45 * alpha),
		offset: Vector::new(0.0, 6.0),
		blur_radius: 18.0,
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

/// Monospaced numeric readout (timecode, frame counters).
pub fn mono_label<'a>(label: impl text::IntoFragment<'a>, size: f32) -> text::Text<'a> {
	ui_label(label, size).font(iced::Font::MONOSPACE)
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

/// Hairline separator.
pub fn separator<'a, Message: 'a>() -> Element<'a, Message> {
	rule::horizontal(1).style(separator_style).into()
}

/// Vertical hairline separator for toolbars.
pub fn vertical_separator<'a, Message: 'a>(height: f32) -> Element<'a, Message> {
	container(rule::vertical(1).style(separator_style))
		.height(height)
		.into()
}

pub fn separator_style(_theme: &Theme) -> rule::Style {
	rule::Style {
		color: BORDER_SUBTLE_COLOR,
		radius: 0.0.into(),
		fill_mode: rule::FillMode::Full,
		snap: true,
	}
}

/// Wrap `content` in a delayed, Apple-style tooltip.
pub fn tip<'a, Message: 'a>(
	content: impl Into<Element<'a, Message>>,
	label: impl text::IntoFragment<'a>,
	position: tooltip_widget::Position,
) -> Element<'a, Message> {
	tooltip_widget(
		content,
		container(ui_label(label, FONT_UI).color(TEXT_PRIMARY_COLOR)).padding([5, 8]),
		position,
	)
	.gap(6)
	.delay(std::time::Duration::from_millis(550))
	.style(tooltip)
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

/// Panel interior fill without chrome — use inside [`panel_system`] areas.
pub fn panel_surface(_theme: &Theme) -> container::Style {
	container::Style {
		background: Some(PANEL_COLOR.into()),
		text_color: Some(TEXT_PRIMARY_COLOR),
		..Default::default()
	}
}

/// Transparent surface that only sets text color (content over a canvas).
pub fn transparent(_theme: &Theme) -> container::Style {
	container::Style {
		text_color: Some(TEXT_PRIMARY_COLOR),
		..Default::default()
	}
}

/// Header strip inside a panel (sub-toolbars, viewer bars).
pub fn toolbar(_theme: &Theme) -> container::Style {
	container::Style {
		background: Some(PANEL_COLOR.into()),
		text_color: Some(TEXT_SECONDARY_COLOR),
		..Default::default()
	}
}

/// Recessed well (viewer backdrop, empty states).
pub fn well(_theme: &Theme) -> container::Style {
	container::Style {
		background: Some(WELL_COLOR.into()),
		text_color: Some(TEXT_SECONDARY_COLOR),
		..Default::default()
	}
}

/// Section card inside the inspector.
pub fn card(_theme: &Theme) -> container::Style {
	container::Style {
		background: Some(white(0.025).into()),
		text_color: Some(TEXT_PRIMARY_COLOR),
		border: Border {
			color: white(0.05),
			width: BORDER_THICKNESS,
			radius: 8.0.into(),
		},
		..Default::default()
	}
}

/// Small rounded tag (kind badges, resolution chips).
pub fn chip(_theme: &Theme) -> container::Style {
	container::Style {
		background: Some(white(0.07).into()),
		text_color: Some(TEXT_SECONDARY_COLOR),
		border: rounded(CHIP_RADIUS),
		..Default::default()
	}
}

/// Colored chip, e.g. a label swatch.
pub fn chip_tinted(color: Color) -> impl Fn(&Theme) -> container::Style {
	move |_theme| container::Style {
		background: Some(Color { a: 0.18, ..color }.into()),
		text_color: Some(color),
		border: rounded(CHIP_RADIUS),
		..Default::default()
	}
}

/// Solid swatch for color previews.
pub fn swatch(color: Color) -> impl Fn(&Theme) -> container::Style {
	move |_theme| container::Style {
		background: Some(color.into()),
		border: Border {
			color: white(0.22),
			width: BORDER_THICKNESS,
			radius: CHIP_RADIUS.into(),
		},
		..Default::default()
	}
}

/// Track behind a segmented control; the selection pill is drawn on top.
pub fn segmented_track(_theme: &Theme) -> container::Style {
	container::Style {
		background: Some(white(0.05).into()),
		border: Border {
			color: white(0.04),
			width: BORDER_THICKNESS,
			radius: 7.0.into(),
		},
		..Default::default()
	}
}

/// Floating heads-up display (toasts, viewer HUD).
pub fn hud(alpha: f32) -> impl Fn(&Theme) -> container::Style {
	move |_theme| container::Style {
		background: Some(
			Color {
				a: 0.94 * alpha,
				..RAISED_COLOR
			}
			.into(),
		),
		text_color: Some(TEXT_PRIMARY_COLOR.scale_alpha(alpha)),
		border: Border {
			color: white(0.10 * alpha),
			width: BORDER_THICKNESS,
			radius: 12.0.into(),
		},
		shadow: float_shadow(alpha),
		snap: true,
	}
}

pub fn tooltip(_theme: &Theme) -> container::Style {
	container::Style {
		background: Some(RAISED_COLOR.into()),
		text_color: Some(TEXT_PRIMARY_COLOR),
		border: Border {
			color: white(0.10),
			width: BORDER_THICKNESS,
			radius: 6.0.into(),
		},
		shadow: float_shadow(0.8),
		snap: true,
	}
}

pub fn list_row(selected: bool) -> impl Fn(&Theme) -> container::Style {
	move |_theme| container::Style {
		background: selected.then(|| BACKGROUND_ACTIVE_COLOR.into()),
		text_color: Some(if selected {
			TEXT_PRIMARY_COLOR_INVERTED
		} else {
			TEXT_PRIMARY_COLOR
		}),
		border: rounded(BORDER_RADIUS),
		..Default::default()
	}
}

// ---------------------------------------------------------------------------
// Buttons
// ---------------------------------------------------------------------------

/// Filled control button (secondary actions).
pub fn button_tool(_theme: &Theme, status: button::Status) -> button::Style {
	let (background, text_color) = match status {
		button::Status::Active => (WIDGET_COLOR, TEXT_PRIMARY_COLOR),
		button::Status::Hovered => (WIDGET_HOVER_COLOR, TEXT_PRIMARY_COLOR),
		button::Status::Pressed => (WIDGET_PRESSED_COLOR, TEXT_PRIMARY_COLOR),
		button::Status::Disabled => (Color { a: 0.5, ..WIDGET_COLOR }, TEXT_MUTED_COLOR),
	};

	button::Style {
		background: Some(background.into()),
		text_color,
		border: rounded(BORDER_RADIUS),
		shadow: Shadow::default(),
		snap: true,
	}
}

/// Transparent until hover — icon buttons, header actions.
pub fn button_ghost(_theme: &Theme, status: button::Status) -> button::Style {
	let (background, text_color) = match status {
		button::Status::Active => (None, TEXT_SECONDARY_COLOR),
		button::Status::Hovered => (Some(white(0.07).into()), TEXT_PRIMARY_COLOR),
		button::Status::Pressed => (Some(white(0.12).into()), TEXT_PRIMARY_COLOR),
		button::Status::Disabled => (None, TEXT_MUTED_COLOR),
	};

	button::Style {
		background,
		text_color,
		border: rounded(BORDER_RADIUS),
		shadow: Shadow::default(),
		snap: true,
	}
}

/// Toggle button: subtle fill when `active`, ghost otherwise.
pub fn button_toggle(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
	move |theme, status| {
		if !active {
			return button_ghost(theme, status);
		}
		let background = match status {
			button::Status::Hovered => white(0.16),
			button::Status::Pressed => white(0.20),
			_ => white(0.11),
		};
		button::Style {
			background: Some(background.into()),
			text_color: TEXT_PRIMARY_COLOR,
			border: rounded(BORDER_RADIUS),
			shadow: Shadow::default(),
			snap: true,
		}
	}
}

/// Prominent accent button (primary action).
pub fn button_accent(_theme: &Theme, status: button::Status) -> button::Style {
	let background = match status {
		button::Status::Active => ACCENT_COLOR,
		button::Status::Hovered => ACCENT_BRIGHT_COLOR,
		button::Status::Pressed => mix(ACCENT_COLOR, Color::BLACK, 0.15),
		button::Status::Disabled => Color { a: 0.4, ..ACCENT_COLOR },
	};

	button::Style {
		background: Some(background.into()),
		text_color: Color::WHITE,
		border: rounded(BORDER_RADIUS),
		shadow: Shadow::default(),
		snap: true,
	}
}

/// Segment inside a segmented control. Only text color changes; the sliding
/// selection pill is drawn behind the row.
pub fn button_segment(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
	move |_theme, status| {
		let text_color = match (selected, status) {
			(true, _) => TEXT_PRIMARY_COLOR,
			(false, button::Status::Hovered | button::Status::Pressed) => TEXT_PRIMARY_COLOR,
			(false, _) => TEXT_SECONDARY_COLOR,
		};
		// 押下時の専用スタイルは持たない（選択ピルの移動で反応を示す）
		let background = match (selected, status) {
			(false, button::Status::Hovered | button::Status::Pressed) => Some(white(0.04).into()),
			_ => None,
		};
		button::Style {
			background,
			text_color,
			border: rounded(6.0),
			shadow: Shadow::default(),
			snap: true,
		}
	}
}

/// Editor-type selector: transparent chip that lights up on hover.
pub fn button_editor_type(_theme: &Theme, status: button::Status) -> button::Style {
	let background = match status {
		button::Status::Active => None,
		button::Status::Hovered => Some(white(0.07).into()),
		button::Status::Pressed => Some(white(0.12).into()),
		button::Status::Disabled => None,
	};

	button::Style {
		background,
		text_color: TEXT_PRIMARY_COLOR,
		border: rounded(BORDER_RADIUS),
		shadow: Shadow::default(),
		snap: true,
	}
}

/// Editor-type menu row. `selected` marks the current editor.
pub fn button_menu_item(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
	button_menu_item_faded(selected, 1.0)
}

/// Menu row with alpha for open/close animation. Hover uses the macOS-style
/// accent highlight; the current item keeps a quiet fill.
pub fn button_menu_item_faded(selected: bool, alpha: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
	move |_theme, status| {
		let (background, text_color) = match status {
			button::Status::Hovered | button::Status::Pressed => (
				Some(ACCENT_COLOR.scale_alpha(alpha).into()),
				TEXT_PRIMARY_COLOR_INVERTED.scale_alpha(alpha),
			),
			_ if selected => (Some(white(0.08 * alpha).into()), TEXT_PRIMARY_COLOR.scale_alpha(alpha)),
			_ => (None, TEXT_PRIMARY_COLOR.scale_alpha(alpha)),
		};

		button::Style {
			background,
			text_color,
			border: rounded(BORDER_RADIUS),
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
		background: Some(
			Color {
				a: 0.97 * alpha,
				..RAISED_COLOR
			}
			.into(),
		),
		text_color: Some(TEXT_PRIMARY_COLOR.scale_alpha(alpha)),
		border: Border {
			color: white(0.10 * alpha),
			width: BORDER_THICKNESS,
			radius: 9.0.into(),
		},
		shadow: float_shadow(alpha),
		snap: true,
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
		border: rounded(BORDER_RADIUS),
	}
}

pub fn menu(_theme: &Theme) -> menu::Style {
	menu::Style {
		background: RAISED_COLOR.into(),
		border: Border {
			color: white(0.10),
			width: BORDER_THICKNESS,
			radius: 8.0.into(),
		},
		text_color: TEXT_PRIMARY_COLOR,
		selected_text_color: TEXT_PRIMARY_COLOR_INVERTED,
		selected_background: ACCENT_COLOR.into(),
		shadow: float_shadow(1.0),
	}
}

// ---------------------------------------------------------------------------
// Text input
// ---------------------------------------------------------------------------

pub fn text_input(_theme: &Theme, status: text_input_widget::Status) -> text_input_widget::Style {
	let (background, border_color) = match status {
		text_input_widget::Status::Active => (WIDGET_COLOR, Color::TRANSPARENT),
		text_input_widget::Status::Hovered => (WIDGET_HOVER_COLOR, Color::TRANSPARENT),
		text_input_widget::Status::Focused { .. } => (WELL_COLOR, ACCENT_COLOR),
		text_input_widget::Status::Disabled => (Color { a: 0.5, ..WIDGET_COLOR }, Color::TRANSPARENT),
	};

	text_input_widget::Style {
		background: Background::Color(background),
		border: widget_border(border_color),
		icon: TEXT_SECONDARY_COLOR,
		placeholder: TEXT_MUTED_COLOR,
		value: TEXT_PRIMARY_COLOR,
		selection: Color {
			a: 0.45,
			..ACCENT_COLOR
		},
	}
}

/// Flat title field that only reveals its chrome on hover / focus
/// (inline rename).
pub fn title_input(theme: &Theme, status: text_input_widget::Status) -> text_input_widget::Style {
	let base = text_input(theme, status);
	match status {
		text_input_widget::Status::Active => text_input_widget::Style {
			background: Background::Color(Color::TRANSPARENT),
			..base
		},
		text_input_widget::Status::Hovered => text_input_widget::Style {
			background: Background::Color(white(0.05)),
			..base
		},
		_ => base,
	}
}

/// Search field: pill shaped, quieter than a value field.
pub fn search_input(theme: &Theme, status: text_input_widget::Status) -> text_input_widget::Style {
	let base = text_input(theme, status);
	text_input_widget::Style {
		background: match status {
			text_input_widget::Status::Focused { .. } => base.background,
			_ => Background::Color(white(0.06)),
		},
		border: Border {
			radius: 7.0.into(),
			..base.border
		},
		..base
	}
}

// ---------------------------------------------------------------------------
// Slider — thin rail, round white knob
// ---------------------------------------------------------------------------

pub fn slider(_theme: &Theme, status: slider_widget::Status) -> slider_widget::Style {
	let knob = match status {
		slider_widget::Status::Active => Color::from_rgb8(220, 220, 226),
		slider_widget::Status::Hovered | slider_widget::Status::Dragged => Color::WHITE,
	};

	slider_widget::Style {
		rail: slider_widget::Rail {
			backgrounds: (ACCENT_COLOR.into(), white(0.12).into()),
			width: 3.0,
			border: Border {
				radius: Radius::new(1.5),
				width: 0.0,
				color: Color::TRANSPARENT,
			},
		},
		handle: slider_widget::Handle {
			shape: slider_widget::HandleShape::Circle { radius: 6.0 },
			background: knob.into(),
			border_width: 0.5,
			border_color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
		},
	}
}

// ---------------------------------------------------------------------------
// Scrollable — overlay-style thin scrollers
// ---------------------------------------------------------------------------

pub fn scrollable(_theme: &Theme, status: scrollable_widget::Status) -> scrollable_widget::Style {
	let rail = |scroller_color: Color| scrollable_widget::Rail {
		background: None,
		border: Border::default(),
		scroller: scrollable_widget::Scroller {
			background: scroller_color.into(),
			border: rounded(3.0),
		},
	};

	let base = rail(white(0.16));
	let hot = rail(white(0.34));

	let auto_scroll = scrollable_widget::AutoScroll {
		background: Color {
			a: 0.75,
			..RAISED_COLOR
		}
		.into(),
		border: rounded(BORDER_RADIUS),
		shadow: Shadow::default(),
		icon: TEXT_PRIMARY_COLOR,
	};

	let (vertical_hot, horizontal_hot) = match status {
		scrollable_widget::Status::Active { .. } => (false, false),
		scrollable_widget::Status::Hovered {
			is_horizontal_scrollbar_hovered,
			is_vertical_scrollbar_hovered,
			..
		} => (is_vertical_scrollbar_hovered, is_horizontal_scrollbar_hovered),
		scrollable_widget::Status::Dragged {
			is_horizontal_scrollbar_dragged,
			is_vertical_scrollbar_dragged,
			..
		} => (is_vertical_scrollbar_dragged, is_horizontal_scrollbar_dragged),
	};

	scrollable_widget::Style {
		container: container::Style::default(),
		vertical_rail: if vertical_hot { hot } else { base },
		horizontal_rail: if horizontal_hot { hot } else { base },
		gap: None,
		auto_scroll,
	}
}
