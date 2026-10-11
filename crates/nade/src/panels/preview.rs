//! Viewer panel: composition header, viewport with overlays and a
//! transport bar.

use constants::icons::{Icon, icon};
use constants::style::{
	FONT_DISPLAY, FONT_TINY, FONT_UI, ICON_SIZE, SPACE_1, SPACE_2, SPACE_4, TEXT_MUTED_COLOR, TEXT_PRIMARY_COLOR,
	TEXT_SECONDARY_COLOR,
};
use constants::widgets::{self, button_body_center, mono_label, ui_label};
use core::{InstanceId, PreviewModel};
use iced::widget::{Canvas, Space, button, column, container, pick_list, row, shader, stack, tooltip};
use iced::{Alignment, Element, Length, Size, Vector};
use panel_system::PanelSystemMessage;

use crate::message::{AppPanelMessage, PreviewMessage, TransportMessage};
use crate::panel_content::PanelContent;
use crate::tool::Tool;
use crate::viewer::{CompositionSummary, Resolution, ViewerSettings};
use crate::widgets::play_glyph::PlayGlyph;
use crate::widgets::video_view::VideoView;
use crate::widgets::viewer_overlay::{LayerBox, PlaybackHud, ViewerOverlay};
use crate::widgets::viewport::Viewport;

const BAR_HEIGHT: f32 = 32.0;
const TRANSPORT_HEIGHT: f32 = 44.0;

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
	pub pause_amount: f32,
	pub hud: Option<PlaybackHud>,
	pub loop_playback: bool,
}

pub fn view<'a>(props: ViewerProps<'a>) -> Element<'a, PanelSystemMessage<PanelContent, AppPanelMessage>> {
	let content: Element<'a, PreviewMessage> = column![header(&props), viewport(&props), transport(&props)].into();
	content.map(|message| PanelSystemMessage::AppMessage(AppPanelMessage::Preview(message)))
}

fn header<'a>(props: &ViewerProps<'a>) -> Element<'a, PreviewMessage> {
	let title: Element<'a, PreviewMessage> = match &props.composition {
		Some(comp) => row![
			icon(Icon::Comp, ICON_SIZE, TEXT_SECONDARY_COLOR),
			ui_label(comp.name, FONT_UI).color(TEXT_PRIMARY_COLOR),
			container(mono_label(format!("{}×{}", comp.width, comp.height), FONT_TINY).color(TEXT_MUTED_COLOR))
				.padding([2, 5])
				.style(widgets::chip),
		]
		.spacing(SPACE_2 + 2.0)
		.align_y(Alignment::Center)
		.into(),
		None => ui_label("No composition", FONT_UI).color(TEXT_MUTED_COLOR).into(),
	};

	let magnification = props.composition.as_ref().map(|comp| {
		Viewport::new(
			props.viewport_size,
			Size::new(comp.width as f32, comp.height as f32),
			props.preview.view_zoom,
			Vector::ZERO,
		)
		.magnification()
	});
	let zoom_label = magnification.map_or_else(|| "—".to_string(), |m| format!("{:.0}%", m * 100.0));

	let tools = row![
		toggle(
			Icon::Grid,
			props.settings.show_grid,
			"Rule-of-Thirds Grid",
			PreviewMessage::ToggleGrid
		),
		toggle(
			Icon::SafeArea,
			props.settings.show_safe_areas,
			"Title/Action Safe",
			PreviewMessage::ToggleSafeAreas
		),
		widgets::vertical_separator(14.0),
		ghost_icon(Icon::Minus, "Zoom Out", PreviewMessage::ZoomBy(1.0 / 1.25)),
		widgets::tip(
			button(button_body_center(mono_label(zoom_label, FONT_TINY + 1.0)))
				.padding(0)
				.width(46)
				.height(24)
				.style(widgets::button_ghost)
				.on_press(PreviewMessage::ActualPixels),
			"Actual Pixels (100%)",
			tooltip::Position::Bottom,
		),
		ghost_icon(Icon::Plus, "Zoom In", PreviewMessage::ZoomBy(1.25)),
		ghost_icon(Icon::Fit, "Fit in Window", PreviewMessage::FitView),
	]
	.spacing(SPACE_1)
	.align_y(Alignment::Center);

	container(
		row![title, Space::new().width(Length::Fill), tools]
			.align_y(Alignment::Center)
			.height(Length::Fill),
	)
	.width(Length::Fill)
	.height(BAR_HEIGHT)
	.padding([0.0, SPACE_4])
	.into()
}

fn viewport<'a>(props: &ViewerProps<'a>) -> Element<'a, PreviewMessage> {
	let frame_size = props.composition.as_ref().map_or(Size::new(1920.0, 1080.0), |comp| {
		Size::new(comp.width as f32, comp.height as f32)
	});

	let overlay = Canvas::new(ViewerOverlay {
		frame: frame_size,
		zoom: props.preview.view_zoom,
		offset: Vector::new(props.preview.view_offset[0], props.preview.view_offset[1]),
		tool: props.tool,
		layers: props.layers.clone(),
		selected: props.selected,
		show_grid: props.settings.show_grid,
		show_safe_areas: props.settings.show_safe_areas,
		hud: props.hud,
	})
	.width(Length::Fill)
	.height(Length::Fill);

	let picture: Element<'a, PreviewMessage> = match &props.preview.frame {
		Some(frame) => shader(VideoView::new(
			Some(frame.clone()),
			props.preview.view_zoom,
			props.preview.view_offset,
		))
		.width(Length::Fill)
		.height(Length::Fill)
		.into(),
		None => container(ui_label("No Signal", FONT_UI).color(TEXT_MUTED_COLOR))
			.center(Length::Fill)
			.into(),
	};

	container(stack![picture, overlay].width(Length::Fill).height(Length::Fill))
		.width(Length::Fill)
		.height(Length::Fill)
		.style(widgets::well)
		.into()
}

fn transport<'a>(props: &ViewerProps<'a>) -> Element<'a, PreviewMessage> {
	let fps = props.composition.as_ref().map_or(30.0, |comp| comp.fps);
	let duration = props.composition.as_ref().map_or(0.0, |comp| comp.duration);
	let playing = props.preview.is_playing;

	let timecode = row![
		mono_label(core::timecode::format(props.preview.time, fps), FONT_DISPLAY).color(TEXT_PRIMARY_COLOR),
		mono_label(format!("/ {}", core::timecode::format(duration, fps)), FONT_TINY + 1.0).color(TEXT_MUTED_COLOR),
	]
	.spacing(SPACE_2 + 2.0)
	.align_y(Alignment::Center);

	let play = widgets::tip(
		button(button_body_center(
			Canvas::new(PlayGlyph {
				pause_amount: props.pause_amount,
				color: TEXT_PRIMARY_COLOR,
			})
			.width(18)
			.height(18),
		))
		.padding(0)
		.width(40)
		.height(30)
		.style(widgets::button_tool)
		.on_press(PreviewMessage::Transport(TransportMessage::TogglePlay)),
		if playing { "Pause  Space" } else { "Play  Space" },
		tooltip::Position::Top,
	);

	let controls = row![
		transport_button(Icon::SkipStart, "Go to Start  Home", TransportMessage::GoToStart),
		transport_button(Icon::StepBack, "Previous Frame  ←", TransportMessage::StepBackward),
		play,
		transport_button(Icon::StepForward, "Next Frame  →", TransportMessage::StepForward),
		transport_button(Icon::SkipEnd, "Go to End  End", TransportMessage::GoToEnd),
		Space::new().width(SPACE_2),
		widgets::tip(
			button(button_body_center(icon(
				Icon::Loop,
				ICON_SIZE,
				if props.loop_playback {
					constants::style::ACCENT_BRIGHT_COLOR
				} else {
					TEXT_SECONDARY_COLOR
				},
			)))
			.padding(0)
			.width(30)
			.height(26)
			.style(widgets::button_toggle(props.loop_playback))
			.on_press(PreviewMessage::Transport(TransportMessage::ToggleLoop)),
			"Loop Playback",
			tooltip::Position::Top,
		),
	]
	.spacing(SPACE_1)
	.align_y(Alignment::Center);

	let resolution = row![
		ui_label("Resolution", FONT_TINY + 1.0).color(TEXT_MUTED_COLOR),
		pick_list(
			Resolution::ALL,
			Some(props.settings.resolution),
			PreviewMessage::SetResolution
		)
		.text_size(FONT_UI)
		.padding([3, 8])
		.width(86)
		.style(widgets::pick_list)
		.menu_style(widgets::menu),
	]
	.spacing(SPACE_2 + 2.0)
	.align_y(Alignment::Center);

	container(
		stack![
			row![timecode, Space::new().width(Length::Fill), resolution]
				.align_y(Alignment::Center)
				.height(Length::Fill),
			container(controls)
				.width(Length::Fill)
				.height(Length::Fill)
				.center_x(Length::Fill)
				.center_y(Length::Fill),
		]
		.width(Length::Fill)
		.height(Length::Fill),
	)
	.width(Length::Fill)
	.height(TRANSPORT_HEIGHT)
	.padding([0.0, SPACE_4 + 2.0])
	.into()
}

fn transport_button<'a>(glyph: Icon, label: &'a str, message: TransportMessage) -> Element<'a, PreviewMessage> {
	widgets::tip(
		button(button_body_center(icon(glyph, ICON_SIZE, TEXT_SECONDARY_COLOR)))
			.padding(0)
			.width(30)
			.height(26)
			.style(widgets::button_ghost)
			.on_press(PreviewMessage::Transport(message)),
		label,
		tooltip::Position::Top,
	)
}

fn toggle<'a>(glyph: Icon, active: bool, label: &'a str, message: PreviewMessage) -> Element<'a, PreviewMessage> {
	let color = if active {
		constants::style::ACCENT_BRIGHT_COLOR
	} else {
		TEXT_SECONDARY_COLOR
	};
	widgets::tip(
		button(button_body_center(icon(glyph, ICON_SIZE, color)))
			.padding(0)
			.width(26)
			.height(24)
			.style(widgets::button_toggle(active))
			.on_press(message),
		label,
		tooltip::Position::Bottom,
	)
}

fn ghost_icon<'a>(glyph: Icon, label: &'a str, message: PreviewMessage) -> Element<'a, PreviewMessage> {
	widgets::tip(
		button(button_body_center(icon(
			glyph,
			ICON_SIZE_SMALL_PLUS,
			TEXT_SECONDARY_COLOR,
		)))
		.padding(0)
		.width(24)
		.height(24)
		.style(widgets::button_ghost)
		.on_press(message),
		label,
		tooltip::Position::Bottom,
	)
}

const ICON_SIZE_SMALL_PLUS: f32 = 13.0;
