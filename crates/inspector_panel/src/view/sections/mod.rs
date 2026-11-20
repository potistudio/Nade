//! Collapsible section frame and the sections of a layer page.

pub(crate) mod effect_block;
pub(crate) mod effect_param;
pub(crate) mod effects;
pub(crate) mod fill;
pub(crate) mod footage;
pub(crate) mod layer;
pub(crate) mod solid;
pub(crate) mod text;
pub(crate) mod transform;

use iced::time::Instant;
use iced::widget::{button, column, container, row};
use iced::{Alignment, Element, Length};
use nade_constants::icons::{Icon, icon, icon_transformed};
use nade_constants::style::{FONT_UI, SPACE_1, SPACE_2, SPACE_3, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR};
use nade_constants::widgets::{self, ui_label};

use crate::state::InspectorUi;
use crate::view::ROW_HEIGHT;
use crate::{InspectorMessage, Section};

pub(crate) fn section<'a>(
	kind: Section,
	glyph: Icon,
	title: &'a str,
	trailing: Option<Element<'a, InspectorMessage>>,
	ui: &InspectorUi,
	now: Instant,
	body: Element<'a, InspectorMessage>,
) -> Element<'a, InspectorMessage> {
	let openness = ui.disclosure(kind).interpolate(0.0, 1.0, now);

	let toggle = button(widgets::button_body(
		row![
			icon_transformed(
				Icon::ChevronRight,
				10.0,
				TEXT_SECONDARY_COLOR,
				openness * std::f32::consts::FRAC_PI_2,
				1.0,
			),
			icon(glyph, 13.0, TEXT_SECONDARY_COLOR),
			ui_label(title, FONT_UI + 0.5).color(TEXT_PRIMARY_COLOR),
		]
		.spacing(SPACE_3 + 1.0)
		.align_y(Alignment::Center),
	))
	.padding([0.0, SPACE_2])
	.width(Length::Fill)
	.height(ROW_HEIGHT)
	.style(widgets::button_ghost)
	.on_press(InspectorMessage::ToggleSection(kind));

	let mut header = row![toggle].spacing(SPACE_1).align_y(Alignment::Center);
	if let Some(trailing) = trailing {
		header = header.push(trailing);
	}

	let mut content = column![header].spacing(SPACE_2);
	if ui.is_open(kind) {
		content = content.push(container(body).padding([SPACE_1, SPACE_2]));
	}

	widgets::card(container(content).padding([SPACE_2, SPACE_2]).width(Length::Fill))
}
