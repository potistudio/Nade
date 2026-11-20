//! Background export of a composition to a movie file.

use domain::Composition;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Default)]
struct Shared {
	done: AtomicU32,
	total: AtomicU32,
	cancel: AtomicBool,
	result: Mutex<Option<Result<(), String>>>,
}

/// A running export. Poll it from the UI thread.
pub struct ExportTask {
	path: PathBuf,
	shared: Arc<Shared>,
}

impl ExportTask {
	/// Render `composition` (a copy, so editing can continue) on a worker thread.
	pub fn start(composition: Composition, track_rendered: Vec<bool>, path: PathBuf) -> Self {
		let shared = Arc::new(Shared::default());
		let worker = Arc::clone(&shared);
		let output = path.clone();
		let spawned = thread::Builder::new().name("nade-export".into()).spawn(move || {
			let is_rendered = |track: usize| track_rendered.get(track).copied().unwrap_or(true);
			let settings = renderer::export_settings(&composition, &output, is_rendered);
			worker.total.store(renderer::frame_count(&settings), Ordering::Relaxed);
			let result = renderer::export_composition(&composition, settings, is_rendered, |done, _| {
				worker.done.store(done, Ordering::Relaxed);
				!worker.cancel.load(Ordering::Relaxed)
			});
			if let Ok(mut slot) = worker.result.lock() {
				*slot = Some(result.map_err(|error| format!("{error:#}")));
			}
		});
		if let Err(error) = spawned
			&& let Ok(mut slot) = shared.result.lock()
		{
			*slot = Some(Err(format!("failed to start export: {error}")));
		}
		Self { path, shared }
	}

	pub fn path(&self) -> &std::path::Path {
		&self.path
	}

	/// Fraction of frames written (`0.0..=1.0`).
	pub fn progress(&self) -> f32 {
		let total = self.shared.total.load(Ordering::Relaxed);
		if total == 0 {
			return 0.0;
		}
		self.shared.done.load(Ordering::Relaxed) as f32 / total as f32
	}

	pub fn cancel(&self) {
		self.shared.cancel.store(true, Ordering::Relaxed);
	}

	pub fn is_cancelled(&self) -> bool {
		self.shared.cancel.load(Ordering::Relaxed)
	}

	/// The outcome once the worker has finished.
	pub fn take_result(&self) -> Option<Result<(), String>> {
		self.shared.result.lock().ok()?.take()
	}
}
