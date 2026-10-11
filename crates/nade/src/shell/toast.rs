//! Transient HUD notifications ("Keyframe added", "Imported …").

use constants::icons::{Icon, icon};
use constants::style::{ACCENT_BRIGHT_COLOR, FONT_UI, MOTION_SLOW_MS, SPACE_3};
use constants::widgets::{self, ui_label};
use iced::time::Instant;
use iced::widget::{Space, column, container, float, row};
use iced::{Alignment, Element, Length, Vector};

use crate::message::Message;
use crate::theme::TOAST_HOLD_SECS;

const ENTER_SECS: f32 = MOTION_SLOW_MS as f32 / 1000.0;
const EXIT_SECS: f32 = 0.3;
const RISE_PX: f32 = 10.0;

#[derive(Debug, Clone)]
pub struct Toast {
	pub icon: Icon,
	pub text: String,
	pub shown_at: Instant,
}

impl Toast {
	pub fn new(icon: Icon, text: impl Into<String>) -> Self {
		Self {
			icon,
			text: text.into(),
			shown_at: Instant::now(),
		}
	}

	fn elapsed(&self, now: Instant) -> f32 {
		now.saturating_duration_since(self.shown_at).as_secs_f32()
	}

	pub fn is_active(&self, now: Instant) -> bool {
		self.elapsed(now) < ENTER_SECS + TOAST_HOLD_SECS + EXIT_SECS
	}

	/// `(opacity, vertical offset)` for the current frame.
	fn appearance(&self, now: Instant) -> (f32, f32) {
		let t = self.elapsed(now);
		if t < ENTER_SECS {
			let p = t / ENTER_SECS;
			(ease_out_cubic(p), (1.0 - ease_out_back(p)) * RISE_PX)
		} else if t < ENTER_SECS + TOAST_HOLD_SECS {
			(1.0, 0.0)
		} else {
			let p = ((t - ENTER_SECS - TOAST_HOLD_SECS) / EXIT_SECS).clamp(0.0, 1.0);
			(1.0 - ease_out_cubic(p), p * RISE_PX * 0.4)
		}
	}
}

/// Bottom-centered toast layer, meant to be stacked over the window.
pub fn view<'a>(toast: &'a Toast, now: Instant, bottom_inset: f32) -> Element<'a, Message> {
	let (alpha, offset) = toast.appearance(now);

	let capsule = container(
		row![
			icon(toast.icon, 14.0, ACCENT_BRIGHT_COLOR.scale_alpha(alpha)),
			ui_label(toast.text.as_str(), FONT_UI),
		]
		.spacing(SPACE_3 + 2.0)
		.align_y(Alignment::Center),
	)
	.padding([8, 14])
	.style(widgets::hud(alpha));

	column![
		Space::new().height(Length::Fill),
		container(float(capsule).translate(move |_, _| Vector::new(0.0, offset)))
			.width(Length::Fill)
			.center_x(Length::Fill),
		Space::new().height(bottom_inset),
	]
	.width(Length::Fill)
	.height(Length::Fill)
	.into()
}

fn ease_out_cubic(t: f32) -> f32 {
	1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

fn ease_out_back(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	let c1 = 1.70158;
	let c3 = c1 + 1.0;
	1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}
