//! Viewer panel: composition header, viewport with overlays and a
//! transport bar.

mod header;
mod transport;
mod viewport;

use core::{CompositionSummary, InstanceId, PreviewModel};
use iced::widget::column;
use iced::{Element, Size};

use crate::message::PreviewMessage;
use crate::settings::ViewerSettings;
use crate::tool::Tool;
use crate::widgets::viewer_overlay::LayerBox;

pub(super) const BAR_HEIGHT: f32 = constants::style::ROW_HEIGHT;
pub(super) const TRANSPORT_HEIGHT: f32 = constants::style::ROW_HEIGHT;

pub struct ViewerProps<'a> {
	pub preview: &'a PreviewModel,
	pub composition: Option<CompositionSummary<'a>>,
	pub tool: Tool,
	pub layers: Vec<LayerBox>,
	pub selected: Option<InstanceId>,
	pub settings: ViewerSettings,
	/// Size of the viewport area (last reported by the overlay).
	pub viewport_size: Size,
	/// Morph of the play button (`1.0` = pause glyph).
	pub loop_playback: bool,
}

pub fn view<'a>(props: ViewerProps<'a>) -> Element<'a, PreviewMessage> {
	column![
		header::header(&props),
		viewport::viewport(&props),
		transport::transport(&props)
	]
	.into()
}
