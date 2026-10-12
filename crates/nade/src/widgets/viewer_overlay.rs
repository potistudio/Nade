//! Viewer overlay: guides, selection gizmo and all pointer
//! input of the viewer (tools, pan, zoom). Drawn on top of the video shader.

use constants::style::{ACCENT_BRIGHT_COLOR, ACCENT_COLOR};
use core::InstanceId;
use iced::widget::canvas::{self, Frame, Geometry, LineDash, Path, Program, Stroke};
use iced::{Color, Event, Point, Rectangle, Renderer, Size, Theme, Vector, keyboard, mouse};

use crate::message::PreviewMessage;
use crate::tool::Tool;
use crate::widgets::viewport::Viewport;

const SCROLL_LINE_PX: f32 = 20.0;
const WHEEL_ZOOM_SPEED: f32 = 0.01;
const CLICK_ZOOM_FACTOR: f32 = 1.5;
/// Extra hit area around layer bounds (screen px).
const HIT_SLOP: f32 = 4.0;
const HANDLE_SIZE: f32 = 6.0;

/// Screen-independent bounds of a layer in comp space.
#[derive(Debug, Clone, Copy)]
pub struct LayerBox {
	pub id: InstanceId,
	/// Center relative to the comp center (comp px).
	pub center: [f32; 2],
	/// Half extents after scale (comp px). Zero for layers without content.
	pub half_size: [f32; 2],
	/// Z rotation in degrees (clockwise on screen).
	pub rotation: f32,
}

impl LayerBox {
	fn rotate(&self, local: [f32; 2], sign: f32) -> [f32; 2] {
		let (sin, cos) = (self.rotation.to_radians() * sign).sin_cos();
		[local[0] * cos - local[1] * sin, local[0] * sin + local[1] * cos]
	}

	fn corners(&self) -> [[f32; 2]; 4] {
		let [hw, hh] = self.half_size;
		[[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]].map(|corner| {
			let rotated = self.rotate(corner, 1.0);
			[self.center[0] + rotated[0], self.center[1] + rotated[1]]
		})
	}

	fn contains(&self, comp: [f32; 2], slop: f32) -> bool {
		let local = self.rotate([comp[0] - self.center[0], comp[1] - self.center[1]], -1.0);
		let [hw, hh] = self.half_size;
		if hw <= 0.0 || hh <= 0.0 {
			return local[0].hypot(local[1]) <= slop * 2.0;
		}
		local[0].abs() <= hw + slop && local[1].abs() <= hh + slop
	}
}

#[derive(Debug, Clone)]
pub struct ViewerOverlay {
	pub frame: Size,
	pub zoom: f32,
	pub offset: Vector,
	pub tool: Tool,
	/// Visible layers, back to front.
	pub layers: Vec<LayerBox>,
	pub selected: Option<InstanceId>,
	pub show_grid: bool,
	pub show_safe_areas: bool,
}

#[derive(Debug, Clone, Copy)]
enum Drag {
	Pan { last: Point },
	Move { last: [f32; 2] },
	Rotate { center: Point, last_angle: f32 },
}

#[derive(Debug, Default)]
pub struct OverlayState {
	modifiers: keyboard::Modifiers,
	drag: Option<Drag>,
	/// 最後に通知したビューアのサイズ
	reported_size: Option<Size>,
}

impl ViewerOverlay {
	fn viewport(&self, size: Size) -> Viewport {
		Viewport::new(size, self.frame, self.zoom, self.offset)
	}

	/// Topmost layer under `point` (screen space).
	fn hit_test(&self, viewport: &Viewport, point: Point) -> Option<&LayerBox> {
		let comp = viewport.screen_to_comp(point);
		let slop = HIT_SLOP / viewport.display_scale().max(f32::EPSILON);
		// 選択中のレイヤーを優先し、次に前面から探す
		self.layers
			.iter()
			.filter(|layer| Some(layer.id) == self.selected)
			.chain(self.layers.iter().rev())
			.find(|layer| layer.contains(comp, slop))
	}

	fn selected_layer(&self) -> Option<&LayerBox> {
		self.layers.iter().find(|layer| Some(layer.id) == self.selected)
	}

	fn view_changed(zoom: f32, offset: Vector) -> PreviewMessage {
		PreviewMessage::ViewChanged {
			zoom,
			offset: [offset.x, offset.y],
		}
	}

	fn press_left(
		&self,
		state: &mut OverlayState,
		viewport: &Viewport,
		point: Point,
	) -> Option<canvas::Action<PreviewMessage>> {
		let comp = viewport.screen_to_comp(point);
		match self.tool {
			Tool::Hand => {
				state.drag = Some(Drag::Pan { last: point });
				Some(canvas::Action::capture())
			}
			Tool::Zoom => {
				let factor = if state.modifiers.alt() {
					1.0 / CLICK_ZOOM_FACTOR
				} else {
					CLICK_ZOOM_FACTOR
				};
				let (zoom, offset) = viewport.zoomed_at(factor, point);
				Some(canvas::Action::publish(Self::view_changed(zoom, offset)).and_capture())
			}
			Tool::Text => Some(canvas::Action::publish(PreviewMessage::CreateText { position: comp }).and_capture()),
			Tool::Select => {
				let Some(hit) = self.hit_test(viewport, point) else {
					return Some(canvas::Action::publish(PreviewMessage::SelectLayer(None)).and_capture());
				};
				state.drag = Some(Drag::Move { last: comp });
				Some(Self::select_then_capture(hit.id, self.selected))
			}
			Tool::Rotate => {
				let target = self.hit_test(viewport, point).or_else(|| self.selected_layer())?;
				let center = viewport.comp_to_screen(target.center);
				state.drag = Some(Drag::Rotate {
					center,
					last_angle: (point.y - center.y).atan2(point.x - center.x).to_degrees(),
				});
				Some(Self::select_then_capture(target.id, self.selected))
			}
		}
	}

	fn select_then_capture(id: InstanceId, selected: Option<InstanceId>) -> canvas::Action<PreviewMessage> {
		if selected == Some(id) {
			canvas::Action::capture()
		} else {
			canvas::Action::publish(PreviewMessage::SelectLayer(Some(id))).and_capture()
		}
	}

	fn drag_to(&self, state: &mut OverlayState, viewport: &Viewport, point: Point) -> Option<PreviewMessage> {
		match state.drag? {
			Drag::Pan { last } => {
				state.drag = Some(Drag::Pan { last: point });
				Some(Self::view_changed(
					self.zoom,
					Vector::new(self.offset.x + point.x - last.x, self.offset.y + point.y - last.y),
				))
			}
			Drag::Move { last } => {
				let comp = viewport.screen_to_comp(point);
				state.drag = Some(Drag::Move { last: comp });
				let mut delta = [comp[0] - last[0], comp[1] - last[1]];
				// Shift: 主軸方向だけに拘束
				if state.modifiers.shift() {
					if delta[0].abs() > delta[1].abs() {
						delta[1] = 0.0;
					} else {
						delta[0] = 0.0;
					}
				}
				Some(PreviewMessage::MoveLayer { delta })
			}
			Drag::Rotate { center, last_angle } => {
				let angle = (point.y - center.y).atan2(point.x - center.x).to_degrees();
				let mut delta = angle - last_angle;
				if delta > 180.0 {
					delta -= 360.0;
				} else if delta < -180.0 {
					delta += 360.0;
				}
				state.drag = Some(Drag::Rotate {
					center,
					last_angle: angle,
				});
				Some(PreviewMessage::RotateLayer { degrees: delta })
			}
		}
	}

	fn draw_guides(&self, frame: &mut Frame, rect: Rectangle) {
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

	fn layer_outline(viewport: &Viewport, layer: &LayerBox) -> Option<Path> {
		if layer.half_size[0] <= 0.0 || layer.half_size[1] <= 0.0 {
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

	fn draw_gizmo(&self, frame: &mut Frame, viewport: &Viewport, layer: &LayerBox) {
		if let Some(outline) = Self::layer_outline(viewport, layer) {
			frame.stroke(&outline, Stroke::default().with_color(ACCENT_COLOR).with_width(1.25));
			for corner in layer.corners() {
				let point = viewport.comp_to_screen(corner);
				let handle = Path::rectangle(
					Point::new(point.x - HANDLE_SIZE * 0.5, point.y - HANDLE_SIZE * 0.5),
					Size::new(HANDLE_SIZE, HANDLE_SIZE),
				);
				frame.fill(&handle, Color::WHITE);
				frame.stroke(&handle, Stroke::default().with_color(ACCENT_COLOR).with_width(1.0));
			}
		}

		// アンカー（位置）: 円 + 十字
		let anchor = viewport.comp_to_screen(layer.center);
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
			let scale = viewport.display_scale();
			let radius = layer.half_size[0].hypot(layer.half_size[1]) * scale + 14.0;
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

impl Program<PreviewMessage> for ViewerOverlay {
	type State = OverlayState;

	fn update(
		&self,
		state: &mut OverlayState,
		event: &Event,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> Option<canvas::Action<PreviewMessage>> {
		let viewport = self.viewport(bounds.size());
		// ドラッグ中は領域外でも追従させるため、絶対座標から算出する
		let local = cursor
			.position()
			.map(|position| Point::new(position.x - bounds.x, position.y - bounds.y));
		let inside = cursor.is_over(bounds);

		// サイズ変化をアプリへ伝える（倍率 % 表示のため）
		if state.reported_size != Some(bounds.size()) {
			state.reported_size = Some(bounds.size());
			return Some(canvas::Action::publish(PreviewMessage::ViewportSized(bounds.size())));
		}

		match event {
			Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
				state.modifiers = *modifiers;
				None
			}
			Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) if inside => {
				state.drag = Some(Drag::Pan { last: local? });
				Some(canvas::Action::capture())
			}
			Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if inside => {
				self.press_left(state, &viewport, local?)
			}
			Event::Mouse(mouse::Event::CursorMoved { .. }) => {
				let message = self.drag_to(state, &viewport, local?)?;
				Some(canvas::Action::publish(message).and_capture())
			}
			Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left | mouse::Button::Middle)) => {
				state.drag.take().map(|_| canvas::Action::capture())
			}
			Event::Mouse(mouse::Event::WheelScrolled { delta }) if inside => {
				let point = local?;
				let (dx, dy) = match delta {
					mouse::ScrollDelta::Lines { x, y } => (*x * SCROLL_LINE_PX, *y * SCROLL_LINE_PX),
					mouse::ScrollDelta::Pixels { x, y } => (*x, *y),
				};
				let (zoom, offset) = if state.modifiers.control() || state.modifiers.command() {
					viewport.zoomed_at(1.0 + dy * WHEEL_ZOOM_SPEED, point)
				} else if state.modifiers.shift() {
					(self.zoom, Vector::new(self.offset.x + dy, self.offset.y))
				} else {
					(self.zoom, Vector::new(self.offset.x + dx, self.offset.y + dy))
				};
				Some(canvas::Action::publish(Self::view_changed(zoom, offset)).and_capture())
			}
			_ => None,
		}
	}

	fn draw(
		&self,
		state: &OverlayState,
		renderer: &Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		let mut frame = Frame::new(renderer, bounds.size());
		let viewport = self.viewport(bounds.size());

		self.draw_guides(&mut frame, viewport.frame_rect());

		// ホバー中のレイヤーを薄く縁取る（選択・回転ツールのみ）
		if matches!(self.tool, Tool::Select | Tool::Rotate)
			&& state.drag.is_none()
			&& let Some(point) = cursor.position_in(bounds)
			&& let Some(hovered) = self.hit_test(&viewport, point)
			&& Some(hovered.id) != self.selected
			&& let Some(outline) = Self::layer_outline(&viewport, hovered)
		{
			frame.stroke(
				&outline,
				Stroke::default()
					.with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.45))
					.with_width(1.0),
			);
		}

		if let Some(layer) = self.selected_layer() {
			self.draw_gizmo(&mut frame, &viewport, layer);
		}

		vec![frame.into_geometry()]
	}

	fn mouse_interaction(&self, state: &OverlayState, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
		match state.drag {
			Some(Drag::Pan { .. }) => return mouse::Interaction::Grabbing,
			Some(Drag::Move { .. }) => return mouse::Interaction::Move,
			Some(Drag::Rotate { .. }) => return mouse::Interaction::Grabbing,
			None => {}
		}
		let Some(point) = cursor.position_in(bounds) else {
			return mouse::Interaction::None;
		};
		match self.tool {
			Tool::Hand => mouse::Interaction::Grab,
			Tool::Zoom if state.modifiers.alt() => mouse::Interaction::ZoomOut,
			Tool::Zoom => mouse::Interaction::ZoomIn,
			Tool::Text => mouse::Interaction::Text,
			Tool::Rotate => mouse::Interaction::Crosshair,
			Tool::Select => {
				if self.hit_test(&self.viewport(bounds.size()), point).is_some() {
					mouse::Interaction::Move
				} else {
					mouse::Interaction::Idle
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn layer(rotation: f32) -> LayerBox {
		LayerBox {
			id: InstanceId::new(0),
			center: [100.0, 50.0],
			half_size: [40.0, 10.0],
			rotation,
		}
	}

	#[test]
	fn hit_testing_respects_rotation() {
		assert!(layer(0.0).contains([135.0, 50.0], 0.0));
		assert!(!layer(90.0).contains([135.0, 50.0], 0.0));
		assert!(layer(90.0).contains([100.0, 85.0], 0.0));
	}

	#[test]
	fn empty_layers_hit_near_anchor() {
		let empty = LayerBox {
			half_size: [0.0, 0.0],
			..layer(0.0)
		};
		assert!(empty.contains([103.0, 52.0], 4.0));
		assert!(!empty.contains([130.0, 50.0], 4.0));
	}
}
