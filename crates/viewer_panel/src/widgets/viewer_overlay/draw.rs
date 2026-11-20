//! Grid / safe-area guides and the selection gizmo.

use crate::tool::Tool;
use crate::widgets::viewer_overlay::layer_box::LayerBox;
use crate::widgets::viewer_overlay::{HANDLE_SIZE, HANDLES, ViewerOverlay};
use crate::widgets::viewport::Viewport;
use iced::widget::canvas::{Frame, LineDash, Path, Stroke};
use iced::{Color, Point, Rectangle, Size};
use nade_constants::style::{ACCENT_BRIGHT_COLOR, ACCENT_COLOR};

impl ViewerOverlay {
	pub(super) fn draw_guides(&self, frame: &mut Frame, rect: Rectangle) {
		frame.stroke(
			&Path::rectangle(rect.position(), rect.size()),
			Stroke::default()
				.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.10))
				.with_width(1.0),
		);

		if self.show_grid {
			let stroke = Stroke::default()
				.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.16))
				.with_width(1.0);
			for step in 1..3 {
				let t = step as f32 / 3.0;
				let x = rect.x + rect.width * t;
				let y = rect.y + rect.height * t;
				frame.stroke(
					&Path::line(Point::new(x, rect.y), Point::new(x, rect.y + rect.height)),
					stroke,
				);
				frame.stroke(
					&Path::line(Point::new(rect.x, y), Point::new(rect.x + rect.width, y)),
					stroke,
				);
			}
		}

		if self.show_safe_areas {
			for (inset, alpha) in [(0.05, 0.30), (0.10, 0.18)] {
				let safe = Rectangle {
					x: rect.x + rect.width * inset,
					y: rect.y + rect.height * inset,
					width: rect.width * (1.0 - inset * 2.0),
					height: rect.height * (1.0 - inset * 2.0),
				};
				frame.stroke(
					&Path::rectangle(safe.position(), safe.size()),
					Stroke {
						line_dash: LineDash {
							segments: &[5.0, 4.0],
							offset: 0,
						},
						..Stroke::default()
							.with_color(Color::from_rgba(1.0, 1.0, 1.0, alpha))
							.with_width(1.0)
					},
				);
			}
			let center = rect.center();
			let cross = Stroke::default()
				.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.30))
				.with_width(1.0);
			frame.stroke(
				&Path::line(
					Point::new(center.x - 8.0, center.y),
					Point::new(center.x + 8.0, center.y),
				),
				cross,
			);
			frame.stroke(
				&Path::line(
					Point::new(center.x, center.y - 8.0),
					Point::new(center.x, center.y + 8.0),
				),
				cross,
			);
		}
	}

	pub(super) fn layer_outline(viewport: &Viewport, layer: &LayerBox) -> Option<Path> {
		if !layer.has_area() {
			return None;
		}
		let corners = layer.corners().map(|corner| viewport.comp_to_screen(corner));
		Some(Path::new(|builder| {
			builder.move_to(corners[0]);
			for corner in &corners[1..] {
				builder.line_to(*corner);
			}
			builder.close();
		}))
	}

	pub(super) fn draw_gizmo(&self, frame: &mut Frame, viewport: &Viewport, layer: &LayerBox) {
		if let Some(outline) = Self::layer_outline(viewport, layer) {
			frame.stroke(&outline, Stroke::default().with_color(ACCENT_COLOR).with_width(1.25));
			for handle in HANDLES {
				let point = viewport.comp_to_screen(layer.matrix.apply(layer.handle_local(handle)));
				let handle = Path::rectangle(
					Point::new(point.x - HANDLE_SIZE * 0.5, point.y - HANDLE_SIZE * 0.5),
					Size::new(HANDLE_SIZE, HANDLE_SIZE),
				);
				frame.fill(&handle, Color::WHITE);
				frame.stroke(&handle, Stroke::default().with_color(ACCENT_COLOR).with_width(1.0));
			}
		}

		// アンカーポイント: 円 + 十字
		let anchor = viewport.comp_to_screen(layer.anchor_point());
		let ring = Path::circle(anchor, 5.0);
		frame.fill(&ring, Color::from_rgba(0.0, 0.0, 0.0, 0.35));
		frame.stroke(&ring, Stroke::default().with_color(Color::WHITE).with_width(1.25));
		let cross = Stroke::default().with_color(ACCENT_BRIGHT_COLOR).with_width(1.25);
		frame.stroke(
			&Path::line(
				Point::new(anchor.x - 9.0, anchor.y),
				Point::new(anchor.x + 9.0, anchor.y),
			),
			cross,
		);
		frame.stroke(
			&Path::line(
				Point::new(anchor.x, anchor.y - 9.0),
				Point::new(anchor.x, anchor.y + 9.0),
			),
			cross,
		);

		if self.tool == Tool::Rotate {
			// アンカーから最も遠い角までの距離 + 余白
			let radius = layer
				.corners()
				.into_iter()
				.map(|corner| {
					let corner = viewport.comp_to_screen(corner);
					(corner.x - anchor.x).hypot(corner.y - anchor.y)
				})
				.fold(0.0, f32::max)
				+ 14.0;
			frame.stroke(
				&Path::circle(anchor, radius.max(24.0)),
				Stroke {
					line_dash: LineDash {
						segments: &[3.0, 5.0],
						offset: 0,
					},
					..Stroke::default()
						.with_color(Color {
							a: 0.7,
							..ACCENT_BRIGHT_COLOR
						})
						.with_width(1.25)
				},
			);
		}
	}
}
