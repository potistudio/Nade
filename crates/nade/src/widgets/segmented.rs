//! macOS-style segmented control with a selection pill that glides between
//! segments. The caller owns the pill position (an animated segment index) so
//! the control itself stays stateless.

use constants::widgets::{self, button_body_center, button_segment};
use iced::widget::{Space, button, container, row, stack, tooltip};
use iced::{Border, Color, Element, Length, Shadow, Vector};

/// Inner padding between the track and the segments.
const TRACK_PADDING: f32 = 2.0;

pub struct Segment<'a, T, Message> {
	pub value: T,
	pub content: Element<'a, Message>,
	pub tooltip: Option<String>,
}

impl<'a, T, Message> Segment<'a, T, Message> {
	pub fn new(value: T, content: impl Into<Element<'a, Message>>) -> Self {
		Self {
			value,
			content: content.into(),
			tooltip: None,
		}
	}

	pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
		self.tooltip = Some(tooltip.into());
		self
	}
}

/// Segmented control.
///
/// `pill_position` is the (possibly fractional, mid-animation) index of the
/// selected segment.
pub fn segmented<'a, T, Message>(
	segments: Vec<Segment<'a, T, Message>>,
	selected: T,
	pill_position: f32,
	segment_size: [f32; 2],
	spacing: f32,
	on_select: impl Fn(T) -> Message + 'a,
) -> Element<'a, Message>
where
	T: Copy + PartialEq + 'a,
	Message: Clone + 'a,
{
	let [width, height] = segment_size;
	let count = segments.len();
	let max_index = count.saturating_sub(1) as f32;

	let pill = row![
		Space::new().width(pill_position.clamp(0.0, max_index) * (width + spacing)),
		container(Space::new())
			.width(width)
			.height(height)
			.style(|_theme| container::Style {
				background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.13).into()),
				border: Border {
					color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
					width: 1.0,
					radius: 6.0.into(),
				},
				shadow: Shadow {
					color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
					offset: Vector::new(0.0, 1.0),
					blur_radius: 3.0,
				},
				..Default::default()
			}),
	];

	let buttons = row(segments.into_iter().map(|segment| {
		let is_selected = segment.value == selected;
		let control: Element<'a, Message> = button(button_body_center(segment.content))
			.padding(0)
			.width(width)
			.height(height)
			.style(button_segment(is_selected))
			.on_press(on_select(segment.value))
			.into();

		match segment.tooltip {
			Some(label) => widgets::tip(control, label, tooltip::Position::Bottom),
			None => control,
		}
	}))
	.spacing(spacing);

	// stack は先頭の子でサイズが決まるため、全セグメント分を明示する
	container(
		stack![pill, buttons]
			.width(Length::Fixed(width * count as f32 + spacing * max_index))
			.height(Length::Fixed(height)),
	)
	.padding(TRACK_PADDING)
	.style(widgets::segmented_track)
	.into()
}
