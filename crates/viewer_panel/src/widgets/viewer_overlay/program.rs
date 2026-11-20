//! Canvas program: event routing, drawing order and cursor shape.

use crate::message::PreviewMessage;
use crate::tool::Tool;
use crate::widgets::viewer_overlay::{Drag, OverlayState, SCROLL_LINE_PX, ViewerOverlay, WHEEL_ZOOM_SPEED};
use iced::widget::canvas::{self, Frame, Geometry, Program, Stroke};
use iced::{Color, Event, Point, Rectangle, Renderer, Theme, Vector, keyboard, mouse};

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
			// 右クリック: 下のレイヤーを選択（メニューはアプリ側で開く）
			Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) if inside => {
				let hit = self.hit_test(&viewport, local?).map(|layer| layer.id);
				(hit != self.selected).then(|| canvas::Action::publish(PreviewMessage::SelectLayer(hit)))
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
		if matches!(self.tool, Tool::Select | Tool::Rotate | Tool::Anchor)
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
			Some(Drag::Scale { handle, .. }) => return Self::handle_cursor(handle),
			Some(Drag::Anchor { .. }) => return mouse::Interaction::Crosshair,
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
			Tool::Rotate | Tool::Anchor => mouse::Interaction::Crosshair,
			Tool::Select => {
				let viewport = self.viewport(bounds.size());
				if let Some((_, handle)) = self.hit_handle(&viewport, point) {
					Self::handle_cursor(handle)
				} else if self.hit_test(&viewport, point).is_some() {
					mouse::Interaction::Move
				} else {
					mouse::Interaction::Idle
				}
			}
		}
	}
}
