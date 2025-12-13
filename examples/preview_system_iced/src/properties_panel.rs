use iced::{
	Color, Element, Length, Padding,
	widget::{
		Column, Row, Space, column, container, horizontal_rule, pick_list, row, scrollable, slider,
		text,
	},
};

use crate::preview_canvas::{EffectParams, EffectType};

// =============================================================================
// 定数
// =============================================================================

mod consts {
	use iced::Color;

	pub const LABEL_WIDTH: f32 = 100.0;

	pub mod colors {
		use super::Color;

		pub const PANEL_BG: Color = Color::from_rgb(0.15, 0.15, 0.15);
		pub const SECTION_BG: Color = Color::from_rgb(0.18, 0.18, 0.18);
		pub const HEADER_BG: Color = Color::from_rgb(0.12, 0.12, 0.12);
		pub const BORDER: Color = Color::from_rgb(0.25, 0.25, 0.25);
		pub const TEXT_SECONDARY: Color = Color::from_rgb(0.6, 0.6, 0.6);
	}
}

use consts::*;

// =============================================================================
// プロパティパネル
// =============================================================================

#[derive(Debug, Clone)]
pub struct PropertiesPanel {
	time_scale: f32,
	color1_r: f32,
	color1_g: f32,
	color1_b: f32,
	color2_r: f32,
	color2_g: f32,
	color2_b: f32,
	effect_type: EffectType,
	param1: f32,
	param2: f32,
	param3: f32,
}

impl PropertiesPanel {
	pub fn new(params: EffectParams) -> Self {
		Self {
			time_scale: params.time_scale,
			color1_r: params.color1.r,
			color1_g: params.color1.g,
			color1_b: params.color1.b,
			color2_r: params.color2.r,
			color2_g: params.color2.g,
			color2_b: params.color2.b,
			effect_type: params.effect_type,
			param1: params.param1,
			param2: params.param2,
			param3: params.param3,
		}
	}

	pub fn get_color1(&self) -> Color {
		Color::from_rgb(self.color1_r, self.color1_g, self.color1_b)
	}

	pub fn get_color2(&self) -> Color {
		Color::from_rgb(self.color2_r, self.color2_g, self.color2_b)
	}

	pub fn update(&mut self, message: PropertiesPanelMessage) {
		match message {
			PropertiesPanelMessage::TimeScaleChanged(v) => self.time_scale = v,
			PropertiesPanelMessage::Color1RChanged(v) => self.color1_r = v,
			PropertiesPanelMessage::Color1GChanged(v) => self.color1_g = v,
			PropertiesPanelMessage::Color1BChanged(v) => self.color1_b = v,
			PropertiesPanelMessage::Color2RChanged(v) => self.color2_r = v,
			PropertiesPanelMessage::Color2GChanged(v) => self.color2_g = v,
			PropertiesPanelMessage::Color2BChanged(v) => self.color2_b = v,
			PropertiesPanelMessage::EffectTypeChanged(e) => self.effect_type = e,
			PropertiesPanelMessage::Param1Changed(v) => self.param1 = v,
			PropertiesPanelMessage::Param2Changed(v) => self.param2 = v,
			PropertiesPanelMessage::Param3Changed(v) => self.param3 = v,
		}
	}

	pub fn view(&self) -> Element<'_, PropertiesPanelMessage> {
		let header = container(text("Effect Properties").size(14))
			.padding(Padding::from([12, 16]))
			.width(Length::Fill)
			.style(|_| container::Style {
				background: Some(colors::HEADER_BG.into()),
				..Default::default()
			});

		// エフェクトタイプセクション
		let effect_section = self.section(
			"Effect Type",
			column![
				pick_list(
					EffectType::all(),
					Some(self.effect_type),
					PropertiesPanelMessage::EffectTypeChanged
				)
				.width(Length::Fill)
				.padding(8),
			],
		);

		// タイムスケールセクション
		let time_section = self.section(
			"Animation",
			column![self.slider_row(
				"Speed",
				self.time_scale,
				0.0,
				3.0,
				PropertiesPanelMessage::TimeScaleChanged
			),]
			.spacing(8),
		);

		// パラメータセクション
		let (param1_label, param2_label, param3_label) = self.get_param_labels();
		let params_section = self.section(
			"Parameters",
			column![
				self.slider_row(
					param1_label,
					self.param1,
					0.1,
					10.0,
					PropertiesPanelMessage::Param1Changed
				),
				self.slider_row(
					param2_label,
					self.param2,
					0.0,
					2.0,
					PropertiesPanelMessage::Param2Changed
				),
				self.slider_row(
					param3_label,
					self.param3,
					0.5,
					5.0,
					PropertiesPanelMessage::Param3Changed
				),
			]
			.spacing(8),
		);

		// カラー1セクション
		let color1_preview =
			container(Space::new(Length::Fixed(40.0), Length::Fixed(20.0))).style(move |_| {
				container::Style {
					background: Some(
						Color::from_rgb(self.color1_r, self.color1_g, self.color1_b).into(),
					),
					border: iced::Border {
						color: colors::BORDER,
						width: 1.0,
						radius: 4.0.into(),
					},
					..Default::default()
				}
			});

		let color1_section = self.section(
			"Color 1",
			column![
				row![
					text("Preview").size(13).width(Length::Fixed(LABEL_WIDTH)),
					color1_preview,
				]
				.spacing(8)
				.align_y(iced::Alignment::Center),
				self.color_slider_row("R", self.color1_r, PropertiesPanelMessage::Color1RChanged),
				self.color_slider_row("G", self.color1_g, PropertiesPanelMessage::Color1GChanged),
				self.color_slider_row("B", self.color1_b, PropertiesPanelMessage::Color1BChanged),
			]
			.spacing(8),
		);

		// カラー2セクション
		let color2_preview =
			container(Space::new(Length::Fixed(40.0), Length::Fixed(20.0))).style(move |_| {
				container::Style {
					background: Some(
						Color::from_rgb(self.color2_r, self.color2_g, self.color2_b).into(),
					),
					border: iced::Border {
						color: colors::BORDER,
						width: 1.0,
						radius: 4.0.into(),
					},
					..Default::default()
				}
			});

		let color2_section = self.section(
			"Color 2",
			column![
				row![
					text("Preview").size(13).width(Length::Fixed(LABEL_WIDTH)),
					color2_preview,
				]
				.spacing(8)
				.align_y(iced::Alignment::Center),
				self.color_slider_row("R", self.color2_r, PropertiesPanelMessage::Color2RChanged),
				self.color_slider_row("G", self.color2_g, PropertiesPanelMessage::Color2GChanged),
				self.color_slider_row("B", self.color2_b, PropertiesPanelMessage::Color2BChanged),
			]
			.spacing(8),
		);

		let scrollable_content = scrollable(
			column![
				effect_section,
				time_section,
				params_section,
				color1_section,
				color2_section,
			]
			.spacing(0),
		)
		.width(Length::Fill)
		.height(Length::Fill);

		column![header, scrollable_content,]
			.width(Length::Fill)
			.height(Length::Fill)
			.into()
	}

	fn get_param_labels(&self) -> (&'static str, &'static str, &'static str) {
		match self.effect_type {
			EffectType::Plasma => ("Frequency", "Amplitude", "Complexity"),
			EffectType::Ripple => ("Frequency", "Speed", "Decay"),
			EffectType::Fire => ("Cooling", "Spread", "Intensity"),
			EffectType::Noise => ("Scale", "Speed", "Octaves"),
			EffectType::Gradient => ("Frequency", "Rotation", "Sharpness"),
		}
	}

	fn section<'a>(
		&self,
		title: &'a str,
		content: Column<'a, PropertiesPanelMessage>,
	) -> Element<'a, PropertiesPanelMessage> {
		let header = container(text(title).size(12).color(colors::TEXT_SECONDARY))
			.padding(Padding::from([8, 12]))
			.width(Length::Fill)
			.style(|_| container::Style {
				background: Some(colors::SECTION_BG.into()),
				..Default::default()
			});

		let body = container(content.spacing(6))
			.padding(12)
			.width(Length::Fill)
			.style(|_| container::Style {
				background: Some(colors::PANEL_BG.into()),
				..Default::default()
			});

		column![header, body, horizontal_rule(1),].into()
	}

	fn slider_row<'a, F>(
		&self,
		label: &'a str,
		value: f32,
		min: f32,
		max: f32,
		on_change: F,
	) -> Row<'a, PropertiesPanelMessage>
	where
		F: Fn(f32) -> PropertiesPanelMessage + 'a,
	{
		row![
			text(label)
				.size(13)
				.color(colors::TEXT_SECONDARY)
				.width(Length::Fixed(LABEL_WIDTH)),
			slider(min..=max, value, on_change)
				.step(0.01)
				.width(Length::Fill),
			text(format!("{:.2}", value))
				.size(11)
				.color(colors::TEXT_SECONDARY)
				.width(Length::Fixed(40.0)),
		]
		.spacing(8)
		.align_y(iced::Alignment::Center)
	}

	fn color_slider_row<'a, F>(
		&self,
		label: &'a str,
		value: f32,
		on_change: F,
	) -> Row<'a, PropertiesPanelMessage>
	where
		F: Fn(f32) -> PropertiesPanelMessage + 'a,
	{
		row![
			text(label)
				.size(13)
				.color(colors::TEXT_SECONDARY)
				.width(Length::Fixed(30.0)),
			slider(0.0..=1.0, value, on_change)
				.step(0.01)
				.width(Length::Fill),
			text(format!("{:.0}", value * 255.0))
				.size(11)
				.color(colors::TEXT_SECONDARY)
				.width(Length::Fixed(30.0)),
		]
		.spacing(8)
		.align_y(iced::Alignment::Center)
	}
}

// =============================================================================
// メッセージ
// =============================================================================

#[derive(Debug, Clone)]
pub enum PropertiesPanelMessage {
	TimeScaleChanged(f32),
	Color1RChanged(f32),
	Color1GChanged(f32),
	Color1BChanged(f32),
	Color2RChanged(f32),
	Color2GChanged(f32),
	Color2BChanged(f32),
	EffectTypeChanged(EffectType),
	Param1Changed(f32),
	Param2Changed(f32),
	Param3Changed(f32),
}
