//! Panel-local UI state: section disclosure, folded effects, links and drafts.

use core::InstanceId;
use iced::Animation;
use iced::time::{Duration, Instant};
use std::collections::HashSet;

use crate::Section;

/// Panel-local UI state owned by the app (section disclosure, links, drafts).
#[derive(Debug)]
pub struct InspectorUi {
	layer_open: Animation<bool>,
	transform_open: Animation<bool>,
	text_open: Animation<bool>,
	solid_open: Animation<bool>,
	footage_open: Animation<bool>,
	effects_open: Animation<bool>,
	/// Effects whose controls are folded, per layer.
	collapsed_effects: HashSet<(InstanceId, u32)>,
	pub scale_linked: bool,
	pub hex_draft: Option<String>,
}

impl Default for InspectorUi {
	fn default() -> Self {
		let disclosure = || {
			Animation::new(true)
				.duration(Duration::from_millis(constants::style::MOTION_FAST_MS + 40))
				.easing(iced::animation::Easing::EaseOutCubic)
		};
		Self {
			layer_open: disclosure(),
			transform_open: disclosure(),
			text_open: disclosure(),
			solid_open: disclosure(),
			footage_open: disclosure(),
			effects_open: disclosure(),
			collapsed_effects: HashSet::new(),
			scale_linked: true,
			hex_draft: None,
		}
	}
}

impl InspectorUi {
	pub(crate) fn disclosure(&self, section: Section) -> &Animation<bool> {
		match section {
			Section::Layer => &self.layer_open,
			Section::Transform => &self.transform_open,
			Section::Text => &self.text_open,
			Section::Solid => &self.solid_open,
			Section::Footage => &self.footage_open,
			Section::Effects => &self.effects_open,
		}
	}

	pub fn toggle(&mut self, section: Section, now: Instant) {
		let animation = match section {
			Section::Layer => &mut self.layer_open,
			Section::Transform => &mut self.transform_open,
			Section::Text => &mut self.text_open,
			Section::Solid => &mut self.solid_open,
			Section::Footage => &mut self.footage_open,
			Section::Effects => &mut self.effects_open,
		};
		let open = animation.value();
		animation.go_mut(!open, now);
	}

	pub fn is_open(&self, section: Section) -> bool {
		self.disclosure(section).value()
	}
	/// Fold / unfold the controls of one effect.
	pub fn toggle_effect(&mut self, layer: InstanceId, effect: u32) {
		if !self.collapsed_effects.remove(&(layer, effect)) {
			self.collapsed_effects.insert((layer, effect));
		}
	}

	/// Unfold an effect (e.g. right after adding it).
	pub fn expand_effect(&mut self, layer: InstanceId, effect: u32) {
		self.collapsed_effects.remove(&(layer, effect));
	}

	pub(crate) fn is_effect_open(&self, layer: InstanceId, effect: u32) -> bool {
		!self.collapsed_effects.contains(&(layer, effect))
	}

	pub fn is_animating(&self, now: Instant) -> bool {
		[
			&self.layer_open,
			&self.transform_open,
			&self.text_open,
			&self.solid_open,
			&self.footage_open,
			&self.effects_open,
		]
		.iter()
		.any(|animation| animation.is_animating(now))
	}
}
