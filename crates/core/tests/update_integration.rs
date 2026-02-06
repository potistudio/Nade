use bytes::Bytes;
use nade_core::{CoreEffect, FrameData, Model, Msg, Transform, update};

fn approx_eq_f32(a: f32, b: f32) {
	assert!((a - b).abs() < 1e-6, "left: {a}, right: {b}");
}

#[test]
fn playback_flow_emits_render_effects_and_updates_time() {
	let mut model = Model::default();
	model.preview.width = 1280;
	model.preview.height = 720;

	let (model, effects) = update(model, Msg::SetTime(2.0));
	assert_eq!(effects.len(), 1);
	match &effects[0] {
		CoreEffect::RenderFrame {
			time,
			width,
			height,
		} => {
			approx_eq_f32(*time, 2.0);
			assert_eq!(*width, 1280);
			assert_eq!(*height, 720);
		}
	}

	let (model, effects) = update(model, Msg::TogglePlay);
	assert!(effects.is_empty());
	assert!(model.preview.is_playing);

	let (model, effects) = update(model, Msg::Tick);
	assert_eq!(effects.len(), 1);
	assert!(model.preview.time > 2.0);

	let (model, effects) = update(model, Msg::TogglePlay);
	assert!(effects.is_empty());
	assert!(!model.preview.is_playing);

	let (model_after_pause_tick, effects) = update(model.clone(), Msg::Tick);
	assert!(effects.is_empty());
	approx_eq_f32(model_after_pause_tick.preview.time, model.preview.time);
}

#[test]
fn rendered_frame_and_transform_persist_in_model() {
	let transform = Transform {
		position: [1.0, 2.0, 3.0],
		rotation: [10.0, 20.0, 30.0],
		scale: [1.0, 0.5, 2.0],
		opacity: 0.7,
	};
	let frame = FrameData {
		width: 4,
		height: 1,
		pixels: Bytes::from_static(&[
			255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
		]),
	};

	let (model, effects) = update(Model::default(), Msg::UpdateTransform(transform));
	assert!(effects.is_empty());
	assert_eq!(model.preview.selection, Some(transform));

	let (model, effects) = update(model, Msg::FrameRendered(frame));
	assert!(effects.is_empty());
	let frame = model
		.preview
		.frame
		.expect("frame should be present after FrameRendered");
	assert_eq!(frame.width, 4);
	assert_eq!(frame.height, 1);
	assert_eq!(frame.pixels.len(), 16);
}
