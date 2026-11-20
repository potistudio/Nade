//! The curve editor widget: property sidebar, toolbar and curve canvas.

mod curve;
mod empty_hint;
mod grid;
mod playhead;
mod program;
mod sidebar;
mod toolbar;

use crate::SIDEBAR_WIDTH;
use crate::message::CurveEditorMessage;
use crate::state::CurveEditorState;
use iced::{
	Element, Length,
	widget::{Space, canvas::Canvas, column, container, row},
};
use nade_constants::style::{BORDER_SUBTLE_COLOR, SPACE_3};
use nade_domain::TransformAnimation;

pub struct CurveEditorWidget<'a> {
	state: &'a CurveEditorState,
	animation: &'a TransformAnimation,
	current_time: f32,
}

impl<'a> CurveEditorWidget<'a> {
	pub fn new(state: &'a CurveEditorState, animation: &'a TransformAnimation, current_time: f32) -> Self {
		Self {
			state,
			animation,
			current_time,
		}
	}

	pub fn view(self) -> Element<'a, CurveEditorMessage> {
		let sidebar = self.sidebar();
		let toolbar = self.toolbar();
		let canvas: Element<'a, CurveEditorMessage> = Canvas::new(self).width(Length::Fill).height(Length::Fill).into();

		row![
			container(sidebar)
				.width(SIDEBAR_WIDTH)
				.height(Length::Fill)
				.padding([SPACE_3, SPACE_3]),
			container(Space::new().width(1).height(Length::Fill)).style(|_| container::Style {
				background: Some(BORDER_SUBTLE_COLOR.into()),
				..Default::default()
			}),
			column![toolbar, canvas].width(Length::Fill).height(Length::Fill),
		]
		.into()
	}
}

impl std::fmt::Debug for CurveEditorWidget<'_> {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.debug_struct("CurveEditorWidget").finish_non_exhaustive()
	}
}
