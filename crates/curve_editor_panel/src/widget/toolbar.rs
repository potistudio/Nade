//! Selected property name, key actions and interpolation buttons.

use crate::TOOLBAR_HEIGHT;
use crate::message::CurveEditorMessage;
use crate::widget::CurveEditorWidget;
use constants::icons::{Icon, icon};
use constants::style::{
	ACCENT_BRIGHT_COLOR, FONT_UI, SPACE_1, SPACE_4, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR, TEXT_SECONDARY_COLOR,
};
use constants::widgets::{self, button_body_center, ui_label};
use core::Interpolation;
use iced::{
	Alignment, Color, Element, Length,
	widget::{Space, button, row, tooltip},
};

impl<'a> CurveEditorWidget<'a> {
	/// Selected property name, key actions and interpolation buttons.
	pub(super) fn toolbar(&self) -> Element<'a, CurveEditorMessage> {
		let interpolation = self
			.state
			.selected_key
			.and_then(|id| self.animation.curve(self.state.selected_property).key(id))
			.map(|key| key.interpolation);
		let has_selection = self.state.selected_key.is_some();

		let icon_button = |glyph: Icon, label: &'static str, active: bool, message: Option<CurveEditorMessage>| {
			let color = if active {
				ACCENT_BRIGHT_COLOR
			} else if message.is_some() {
				TEXT_SECONDARY_COLOR
			} else {
				Color {
					a: 0.35,
					..TEXT_MUTED_COLOR
				}
			};
			widgets::tip(
				button(button_body_center(icon(glyph, 14.0, color)))
					.padding(0)
					.width(28)
					.height(TOOLBAR_HEIGHT)
					.style(widgets::button_toggle(active))
					.on_press_maybe(message),
				label,
				tooltip::Position::Bottom,
			)
		};
		let interpolation_button = |glyph, label, value: Interpolation| {
			icon_button(
				glyph,
				label,
				interpolation == Some(value),
				has_selection.then_some(CurveEditorMessage::SetInterpolation(value)),
			)
		};

		let selected_name = self.state.selected_property.label();
		row![
			ui_label(selected_name, FONT_UI).color(TEXT_PRIMARY_COLOR),
			Space::new().width(Length::Fill),
			icon_button(
				Icon::Keyframe,
				"Add Key at Playhead",
				false,
				Some(CurveEditorMessage::AddKey)
			),
			icon_button(
				Icon::Trash,
				"Delete Key  ⌫",
				false,
				has_selection.then_some(CurveEditorMessage::DeleteSelected)
			),
			widgets::vertical_separator(14.0),
			interpolation_button(Icon::InterpBezier, "Bezier", Interpolation::Bezier),
			interpolation_button(Icon::InterpLinear, "Linear", Interpolation::Linear),
			interpolation_button(Icon::InterpHold, "Hold", Interpolation::Hold),
			widgets::vertical_separator(14.0),
			icon_button(Icon::Fit, "Frame All", false, Some(CurveEditorMessage::FrameAll)),
		]
		.spacing(SPACE_1)
		.align_y(Alignment::Center)
		.padding([0.0, SPACE_4])
		.height(TOOLBAR_HEIGHT)
		.into()
	}
}
