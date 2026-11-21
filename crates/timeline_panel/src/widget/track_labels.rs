//! Track label column with visibility and solo switches.

use crate::widget::{TimelineWidget, squircle_rect};
use crate::{
	TimelineModel,
	icons::{draw_eye_icon, draw_solo_icon},
	interaction::TimelineInteraction,
};
use iced::{
	Color, Point, Rectangle, Size,
	alignment::Vertical,
	widget::canvas::{self, LineCap, Path, Stroke, Text},
};
use nade_constants::style::MONOSPACE_FONT;
use nade_constants::timeline::*;
use nade_core::TrackVisibilityDisplay;

impl TimelineWidget<'_> {
	pub(super) fn draw_track_labels(
		&self,
		frame: &mut canvas::Frame,
		state: &TimelineInteraction,
		model: &TimelineModel,
		rect: Rectangle,
	) {
		frame.fill_rectangle(rect.position(), rect.size(), colors::TRACK_LABEL_BG);

		let reorder_preview = self.reorder_preview;
		let insert_before_idx =
			reorder_preview.map(|(from_idx, to_idx)| if to_idx < from_idx { to_idx } else { to_idx + 1 });

		// with_clip はキャンバス原点がずれていると誤ってクリップされるため使わない。
		// はみ出しは後から描くルーラー／コーナーが覆う。
		{
			for (i, track) in model.tracks.iter().enumerate() {
				let y_offset = match insert_before_idx {
					Some(insert_idx) if i >= insert_idx => TRACK_HEIGHT,
					_ => 0.0,
				};
				let y = rect.y + (i as f32 * TRACK_HEIGHT) + state.scroll_offset.y + y_offset;

				if !state.is_track_visible(y, rect) {
					continue;
				}

				let row_rect = Rectangle::new(Point::new(rect.x, y), Size::new(rect.width, TRACK_HEIGHT));
				let bg_color = if state.selected_track_index == Some(i) {
					Color::from_rgba(1.0, 1.0, 1.0, 0.06)
				} else if i % 2 == 0 {
					colors::TRACK_LABEL_BG
				} else {
					colors::TRACK_LABEL_BG_ALT
				};
				frame.fill_rectangle(row_rect.position(), row_rect.size(), bg_color);

				// ラベル色の帯（掴んで並べ替えるハンドルを兼ねる）
				let rendered = model.is_track_rendered(i);
				let mut label = Self::clip_base_color(i, 0);
				if !rendered {
					label.a = 0.35;
				}
				let strip = Rectangle {
					x: rect.x + (TRACK_HANDLE_WIDTH - 4.0) * 0.5,
					y: y + 6.0,
					width: 4.0,
					height: TRACK_HEIGHT - 12.0,
				};
				frame.fill(&squircle_rect(strip, 4.5), label);

				self.draw_visibility_btn(frame, track_visible_btn_rect(rect.x, y), model.visibility_display(i));
				self.draw_solo_btn(frame, track_solo_btn_rect(rect.x, y), track.solo, track.visible);

				// 1 クリップのトラックは AE のレイヤーのようにクリップ名を表示
				let name = match track.clips.as_slice() {
					[only] => only.name.as_str(),
					_ => track.name.as_str(),
				};
				let name_x = track_name_x(rect.x);
				frame.fill_text(Text {
					content: format!("{}", i + 1),
					position: Point::new(name_x, y + TRACK_HEIGHT * 0.5),
					color: colors::TEXT_MUTED,
					size: 10.0.into(),
					font: MONOSPACE_FONT,
					align_y: Vertical::Center,
					..Text::default()
				});
				frame.fill_text(Text {
					content: name.to_string(),
					position: Point::new(name_x + 18.0, y + TRACK_HEIGHT * 0.5),
					color: if rendered {
						colors::TEXT_PRIMARY
					} else {
						colors::TEXT_MUTED
					},
					size: 11.0.into(),
					align_y: Vertical::Center,
					..Text::default()
				});

				self.draw_horizontal_line(
					frame,
					rect.x,
					rect.x + rect.width,
					y + TRACK_HEIGHT,
					colors::BORDER_SUBTLE,
				);
			}

			// 並べ替え中の挿入位置
			if let Some(insert_idx) = insert_before_idx {
				let gap_y = rect.y + (insert_idx as f32 * TRACK_HEIGHT) + state.scroll_offset.y;
				let gap = squircle_rect(
					Rectangle::new(
						Point::new(rect.x + 2.0, gap_y + 2.0),
						Size::new(rect.width - 4.0, TRACK_HEIGHT - 4.0),
					),
					11.25,
				);
				frame.fill(
					&gap,
					Color {
						a: 0.12,
						..colors::SELECTION
					},
				);
				frame.stroke(
					&gap,
					Stroke::default().with_color(colors::SELECTION_STROKE).with_width(1.5),
				);
			}

			// 並べ替え直後のハイライト
			if let Some((track_idx, alpha)) = self.glowing_track {
				let glow_y = rect.y + (track_idx as f32 * TRACK_HEIGHT) + state.scroll_offset.y;
				frame.fill_rectangle(
					Point::new(rect.x, glow_y),
					Size::new(rect.width, TRACK_HEIGHT),
					Color {
						a: alpha * 0.25,
						..colors::SELECTION
					},
				);
			}
		}

		self.draw_vertical_line(frame, rect.x + rect.width, rect.y, rect.y + rect.height, colors::BORDER);
	}

	// -------------------------------------------------------------------------
	// 描画ヘルパー
	// -------------------------------------------------------------------------
	pub(super) fn draw_visibility_btn(
		&self,
		frame: &mut canvas::Frame,
		rect: Rectangle,
		display: TrackVisibilityDisplay,
	) {
		let (icon_color, draw_slash) = match display {
			TrackVisibilityDisplay::Shown => (colors::TEXT_SECONDARY, false),
			TrackVisibilityDisplay::Hidden => (
				Color {
					a: 0.45,
					..colors::TEXT_MUTED
				},
				true,
			),
			// 可視だがソロ除外: アクセント色で第3状態を明示
			TrackVisibilityDisplay::SoloExcluded => (colors::PLAYHEAD, true),
		};

		draw_eye_icon(frame, rect, icon_color);

		if draw_slash {
			let slash = Path::line(
				Point::new(rect.x + 4.0, rect.y + rect.height - 4.0),
				Point::new(rect.x + rect.width - 4.0, rect.y + 4.0),
			);
			frame.stroke(
				&slash,
				Stroke::default()
					.with_color(icon_color)
					.with_width(1.25)
					.with_line_cap(LineCap::Round),
			);
		}
	}

	pub(super) fn draw_solo_btn(&self, frame: &mut canvas::Frame, rect: Rectangle, active: bool, enabled: bool) {
		let icon_color = if !enabled {
			Color {
				a: 0.3,
				..colors::TEXT_MUTED
			}
		} else if active {
			nade_constants::style::WARNING_COLOR
		} else {
			colors::TEXT_MUTED
		};
		draw_solo_icon(frame, rect, icon_color, active && enabled);
	}
}
