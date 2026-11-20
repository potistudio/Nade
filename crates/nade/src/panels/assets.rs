use crate::message::AppPanelMessage;
use crate::panel_content::PanelContent;
use iced::Element;
use nade_asset_browser::{AssetBrowserState, AssetBrowserWidget};
use nade_domain::Project;
use nade_panel_system::PanelSystemMessage;
use std::time::Instant;

/// One spinner turn.
const SPIN_PERIOD_SECS: f32 = 0.9;

pub fn view<'a>(
	project: &'a Project,
	state: &'a AssetBrowserState,
	now: Instant,
) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	// 最初の取り込みの開始時刻を基準にして、全部のスピナーを同じ位相で回す
	let spinner_angle = state.importing.first().map_or(0.0, |pending| {
		let elapsed = now.saturating_duration_since(pending.started).as_secs_f32();
		(elapsed / SPIN_PERIOD_SECS).fract() * std::f32::consts::TAU
	});
	AssetBrowserWidget::new(state.view_model(project))
		.spinner_angle(spinner_angle)
		.view()
		.map(AppPanelMessage::Assets)
		.map(PanelSystemMessage::AppMessage)
}
