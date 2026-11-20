//! Layer edits from the inspector: applied to the selected instance with
//! the panel's context, reporting what the shell should do next.

use crate::message::{EffectEdit, InspectorMessage, PropertyGroup};
use nade_constants::icons::Icon;
use nade_core::{InstanceId, NodeId};
use nade_domain::{Instance, InstanceContent, LayerEffect, TransformProperty};

/// Shell context for applying an inspector edit to a layer.
#[derive(Debug, Clone)]
pub struct EditContext {
	/// Composition time the edit applies at.
	pub time: f32,
	/// Composition fps (key toasts show its timecode).
	pub fps: f32,
	/// Lock X/Y scale together.
	pub scale_linked: bool,
	/// Pending hex color draft, consumed by `HexSubmitted`.
	pub hex_draft: Option<String>,
	/// Composition frame size in pixels (fit math).
	pub frame: [f32; 2],
}

/// What the shell should do after a layer edit.
#[derive(Debug, Default)]
pub struct EditOutcome {
	/// Toast to show (icon, text).
	pub toast: Option<(Icon, String)>,
	/// Playhead time to jump to (key navigation); skips the preview refresh.
	pub jump: Option<f32>,
	/// New layer name; the shell fans it out to the timeline clips.
	pub renamed: Option<String>,
	/// Id of the added effect; the shell expands and reveals the section.
	pub added_effect: Option<u32>,
}

/// Applies a layer edit to the instance, returning the shell work.
///
/// Panel-UI, composition and orchestration messages never reach this
/// function; the shell has already classified them.
pub fn apply_layer_edit(obj: &mut Instance, msg: InspectorMessage, ctx: &EditContext) -> EditOutcome {
	let mut outcome = EditOutcome::default();
	let time = ctx.time;

	match msg {
		InspectorMessage::SetChannel { property, value } => {
			let partner = match property {
				TransformProperty::ScaleX => Some(TransformProperty::ScaleY),
				TransformProperty::ScaleY => Some(TransformProperty::ScaleX),
				_ => None,
			};
			// 縦横比を固定: 変化率をもう一方の軸にも掛ける
			if let (true, Some(partner)) = (ctx.scale_linked, partner) {
				let previous = obj.property_at(property, time);
				let other = obj.property_at(partner, time);
				let other_next = if previous.abs() > f32::EPSILON {
					other * value / previous
				} else {
					value
				};
				obj.set_property_at(partner, time, other_next);
			}
			obj.set_property_at(property, time, value);
		}
		InspectorMessage::ToggleKeyframe(group) => {
			let channels = group.channels();
			let on_key = channels
				.iter()
				.any(|property| obj.animation.curve(*property).key_at(time).is_some());
			for property in channels {
				let has_key = obj.animation.curve(*property).key_at(time).is_some();
				if has_key == on_key {
					obj.toggle_key_at(*property, time);
				}
			}
			let timecode = nade_core::timecode::format(time, ctx.fps);
			outcome.toast = Some(if on_key {
				(Icon::Keyframe, format!("Removed {} key at {timecode}", group.label()))
			} else {
				(Icon::KeyframeFilled, format!("Keyed {} at {timecode}", group.label()))
			});
		}
		InspectorMessage::JumpToKey { group, forward } => {
			let times = group.channels().iter().filter_map(|property| {
				let curve = obj.animation.curve(*property);
				if forward {
					curve.next_key_time(time)
				} else {
					curve.previous_key_time(time)
				}
			});
			outcome.jump = if forward {
				times.reduce(f32::min)
			} else {
				times.reduce(f32::max)
			};
		}
		InspectorMessage::ResetTransform => {
			for group in PropertyGroup::ALL {
				for property in group.channels() {
					obj.set_property_at(*property, time, PropertyGroup::default_value(*property));
				}
			}
			outcome.toast = Some((Icon::Reset, "Transform reset".to_string()));
		}
		InspectorMessage::Rename(name) => {
			obj.name = name.clone();
			outcome.renamed = Some(name);
		}
		InspectorMessage::SetText(text) => {
			if let Some(slot) = obj.content_mut().text_mut() {
				*slot = text;
			}
		}
		InspectorMessage::SetFontSize(size) => {
			if let Some(slot) = obj.content_mut().font_size_mut() {
				*slot = size.max(1.0);
			}
		}
		InspectorMessage::SetTracking(spacing) => {
			if let Some(slot) = obj.content_mut().spacing_mut() {
				*slot = spacing;
			}
		}
		InspectorMessage::SetFillChannel { index, value } => {
			if let Some(color) = obj.content_mut().fill_color_mut()
				&& let Some(slot) = color.get_mut(index)
			{
				*slot = value.clamp(0.0, 1.0);
			}
		}
		InspectorMessage::SetFillRgb(rgb) => {
			if let Some(color) = obj.content_mut().fill_color_mut() {
				color[..3].copy_from_slice(&rgb);
			}
		}
		InspectorMessage::HexSubmitted => {
			if let Some(rgb) = ctx.hex_draft.as_deref().and_then(parse_hex_color)
				&& let Some(color) = obj.content_mut().fill_color_mut()
			{
				color[..3].copy_from_slice(&rgb);
			}
		}
		InspectorMessage::SetSolidSize { width, height } => {
			if let Some((slot_width, slot_height)) = obj.content_mut().solid_size_mut() {
				*slot_width = width.max(1.0);
				*slot_height = height.max(1.0);
			}
		}
		InspectorMessage::FitToComposition => {
			let [width, height] = match obj.content() {
				InstanceContent::Media(source) => source.size(),
				InstanceContent::Solid { width, height, .. } => [*width, *height],
				_ => [0.0, 0.0],
			};
			if width > 0.0 && height > 0.0 {
				let fit = (ctx.frame[0] / width).min(ctx.frame[1] / height);
				obj.set_property_at(TransformProperty::ScaleX, time, fit);
				obj.set_property_at(TransformProperty::ScaleY, time, fit);
				obj.set_property_at(TransformProperty::PositionX, time, 0.0);
				obj.set_property_at(TransformProperty::PositionY, time, 0.0);
				outcome.toast = Some((Icon::Fit, "Fit to composition".to_string()));
			}
		}
		InspectorMessage::SetBlendMode(mode) => obj.blend_mode = mode,
		InspectorMessage::AddEffect(kind) => {
			outcome.added_effect = Some(obj.add_effect(kind));
			outcome.toast = Some((Icon::Sparkle, format!("Added {}", kind.name())));
		}
		InspectorMessage::RemoveAllEffects => {
			let count = obj.effects.len();
			obj.effects.clear();
			outcome.toast = Some((
				Icon::Trash,
				format!("Removed {count} effect{}", if count == 1 { "" } else { "s" }),
			));
		}
		InspectorMessage::Effect { id, edit } => {
			let (toast, jump) = edit_effect(obj, id, edit, time, ctx.fps);
			outcome.toast = toast;
			outcome.jump = jump;
		}
		_ => {}
	}

	outcome
}

/// Applies an effect edit (enable / reset / value / keys / ordering).
fn edit_effect(
	layer: &mut Instance,
	id: u32,
	edit: EffectEdit,
	time: f32,
	fps: f32,
) -> (Option<(Icon, String)>, Option<f32>) {
	let Some(name) = layer.effect(id).map(LayerEffect::name) else {
		return (None, None);
	};
	match edit {
		EffectEdit::Remove => {
			layer.remove_effect(id);
			return (Some((Icon::Trash, format!("Removed {name}"))), None);
		}
		EffectEdit::Move { up } => {
			layer.move_effect(id, up);
			return (None, None);
		}
		_ => {}
	}
	let Some(effect) = layer.effect_mut(id) else {
		return (None, None);
	};
	match edit {
		EffectEdit::ToggleEnabled => {
			effect.enabled = !effect.enabled;
			(None, None)
		}
		EffectEdit::Reset => {
			effect.reset();
			(Some((Icon::Reset, format!("{name} reset"))), None)
		}
		EffectEdit::SetValue { slot, value } => {
			effect.set_value_at(slot, time, value);
			(None, None)
		}
		EffectEdit::ToggleKey { param } => {
			let label = effect.kind.params().get(param).map_or("", |param| param.name);
			let timecode = nade_core::timecode::format(time, fps);
			let toast = if effect.toggle_key_at(param, time) {
				(Icon::KeyframeFilled, format!("Keyed {label} at {timecode}"))
			} else {
				(Icon::Keyframe, format!("Removed {label} key at {timecode}"))
			};
			(Some(toast), None)
		}
		EffectEdit::JumpToKey { param, forward } => (None, effect.adjacent_key_time(param, time, forward)),
		EffectEdit::ToggleExpanded | EffectEdit::Remove | EffectEdit::Move { .. } => (None, None),
	}
}

/// Parses `#RGB` / `#RRGGBB` (with or without `#`) into linear RGB.
fn parse_hex_color(input: &str) -> Option<[f32; 3]> {
	let hex = input.trim().trim_start_matches('#');
	let expanded: String = match hex.len() {
		3 => hex.chars().flat_map(|c| [c, c]).collect(),
		6 => hex.to_string(),
		_ => return None,
	};
	let channel = |index: usize| u8::from_str_radix(&expanded[index..index + 2], 16).ok();
	Some([
		channel(0)? as f32 / 255.0,
		channel(2)? as f32 / 255.0,
		channel(4)? as f32 / 255.0,
	])
}

#[cfg(test)]
mod tests {
	use super::*;

	fn layer() -> Instance {
		Instance::new(InstanceId::new(0), NodeId::new(0), "Layer", 0.0, 5.0)
	}

	fn ctx() -> EditContext {
		EditContext {
			time: 1.0,
			fps: 30.0,
			scale_linked: true,
			hex_draft: None,
			frame: [1920.0, 1080.0],
		}
	}

	/// Channel edits land at the context time.
	#[test]
	fn set_channel_applies_the_value() {
		let mut obj = layer();

		let outcome = apply_layer_edit(
			&mut obj,
			InspectorMessage::SetChannel {
				property: TransformProperty::PositionX,
				value: 12.0,
			},
			&ctx(),
		);

		assert_eq!(obj.property_at(TransformProperty::PositionX, 1.0), 12.0);
		assert_eq!(outcome.toast, None);
	}

	/// Scale-linked edits apply the change ratio to the partner axis too.
	#[test]
	fn set_channel_keeps_aspect_ratio_when_linked() {
		let mut obj = layer();

		apply_layer_edit(
			&mut obj,
			InspectorMessage::SetChannel {
				property: TransformProperty::ScaleX,
				value: 2.0,
			},
			&ctx(),
		);

		assert_eq!(obj.property_at(TransformProperty::ScaleX, 1.0), 2.0);
		assert_eq!(obj.property_at(TransformProperty::ScaleY, 1.0), 2.0);
	}

	/// Unlinked scale edits touch one axis only.
	#[test]
	fn set_channel_leaves_the_partner_alone_when_unlinked() {
		let mut obj = layer();
		let ctx = EditContext {
			scale_linked: false,
			..ctx()
		};

		apply_layer_edit(
			&mut obj,
			InspectorMessage::SetChannel {
				property: TransformProperty::ScaleX,
				value: 4.0,
			},
			&ctx,
		);

		assert_eq!(obj.property_at(TransformProperty::ScaleX, 1.0), 4.0);
		assert_eq!(obj.property_at(TransformProperty::ScaleY, 1.0), 1.0);
	}

	/// Resetting restores every channel to its group default.
	#[test]
	fn reset_transform_restores_the_defaults() {
		let mut obj = layer();
		apply_layer_edit(
			&mut obj,
			InspectorMessage::SetChannel {
				property: TransformProperty::PositionX,
				value: 99.0,
			},
			&ctx(),
		);

		let outcome = apply_layer_edit(&mut obj, InspectorMessage::ResetTransform, &ctx());

		assert_eq!(
			obj.property_at(TransformProperty::PositionX, 1.0),
			PropertyGroup::default_value(TransformProperty::PositionX)
		);
		assert!(outcome.toast.is_some());
	}

	/// The stopwatch keys every channel of the group at once.
	#[test]
	fn toggle_keyframe_keys_the_whole_group() {
		let mut obj = layer();

		let outcome = apply_layer_edit(
			&mut obj,
			InspectorMessage::ToggleKeyframe(PropertyGroup::Position),
			&ctx(),
		);

		for property in PropertyGroup::Position.channels() {
			assert!(obj.animation.curve(*property).key_at(1.0).is_some());
		}
		assert!(outcome.toast.is_some());
	}

	/// Renames land on the instance and are reported for the timeline.
	#[test]
	fn rename_reports_the_new_name() {
		let mut obj = layer();

		let outcome = apply_layer_edit(&mut obj, InspectorMessage::Rename("Renamed".to_string()), &ctx());

		assert_eq!(obj.name, "Renamed");
		assert_eq!(outcome.renamed.as_deref(), Some("Renamed"));
	}

	/// A submitted draft writes the parsed color into the text content.
	#[test]
	fn hex_submitted_consumes_the_draft() {
		let mut obj = layer();
		obj.set_content(InstanceContent::text_default());
		let ctx = EditContext {
			hex_draft: Some("#FF0000".to_string()),
			..ctx()
		};

		apply_layer_edit(&mut obj, InspectorMessage::HexSubmitted, &ctx);

		let InstanceContent::Text { fill_color, .. } = obj.content() else {
			panic!("expected text content");
		};
		assert_eq!(&fill_color[..3], &[1.0, 0.0, 0.0]);
	}

	/// Fit scales a solid so it covers the composition frame.
	#[test]
	fn fit_to_composition_fits_solids() {
		let mut obj = layer();
		obj.set_content(InstanceContent::Solid {
			width: 100.0,
			height: 50.0,
			color: [1.0; 4],
		});

		let outcome = apply_layer_edit(&mut obj, InspectorMessage::FitToComposition, &ctx());

		// 1920 / 100 = 19.2 is smaller than 1080 / 50 = 21.6.
		assert_eq!(obj.property_at(TransformProperty::ScaleX, 1.0), 19.2);
		assert_eq!(obj.property_at(TransformProperty::ScaleY, 1.0), 19.2);
		assert!(outcome.toast.is_some());
	}

	/// Hex colors parse from #RGB and #RRGGBB forms.
	#[test]
	fn parses_hex_colors() {
		assert_eq!(parse_hex_color("#FF0000"), Some([1.0, 0.0, 0.0]));
		assert_eq!(parse_hex_color("0f0"), Some([0.0, 1.0, 0.0]));
		assert_eq!(parse_hex_color("#12345"), None);
		assert_eq!(parse_hex_color("#GG0000"), None);
	}
}
