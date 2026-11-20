//! Properties panel (After Effects "Effect Controls" / Cavalry "Attribute
//! Editor"): layer header, collapsible sections and keyframe-aware rows.

mod composition;
mod empty;
mod header;
mod labelled;
mod layer;
mod sections;

use constants::widgets;
use iced::time::Instant;
use iced::widget::container;
use iced::{Element, Length};

use crate::InspectorMessage;
use crate::state::InspectorUi;
use crate::target::InspectorTarget;
use crate::view::composition::composition_view;
use crate::view::empty::empty_state;
use crate::view::layer::layer_view;

pub(crate) use labelled::labelled;

pub(crate) const LABEL_WIDTH: f32 = 56.0;
/// Effect parameter names are longer than transform labels.
pub(crate) const EFFECT_LABEL_WIDTH: f32 = 92.0;
/// Width of the `[‹][◆][›]` key cluster.
pub(crate) const KEY_CLUSTER_WIDTH: f32 = 42.0;
pub(crate) const ROW_HEIGHT: f32 = constants::style::ROW_HEIGHT;

pub fn view<'a>(target: InspectorTarget<'a>, ui: &'a InspectorUi, now: Instant) -> Element<'a, InspectorMessage> {
	let content: Element<'a, InspectorMessage> = match target {
		InspectorTarget::Empty => empty_state(),
		InspectorTarget::Composition {
			summary,
			layers,
			background,
			nest_targets,
		} => composition_view(summary, layers, background, nest_targets),
		InspectorTarget::Layer(layer) => layer_view(layer, ui, now),
	};

	container(content)
		.width(Length::Fill)
		.height(Length::Fill)
		.style(widgets::panel_surface)
		.into()
}
