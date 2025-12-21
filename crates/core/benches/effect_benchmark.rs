use criterion::{Criterion, black_box, criterion_group, criterion_main};
use nade_core::effects::WaveEffect;
use nade_core::{Effect, RenderContext, RgbColor};

fn benchmark_wave_effect(c: &mut Criterion) {
	let effect = WaveEffect::default();
	let ctx = RenderContext {
		width: 1920,
		height: 1080,
		time: 1.0,
		frame: 60,
	};

	c.bench_function("WaveEffect::apply", |b| {
		b.iter(|| {
			// Benchmark a single pixel application
			// In reality, this is called millions of times per frame.
			effect.apply(
				black_box(RgbColor::BLACK),
				black_box(960),
				black_box(540),
				black_box(&ctx),
			)
		})
	});
}

criterion_group!(benches, benchmark_wave_effect);
criterion_main!(benches);
