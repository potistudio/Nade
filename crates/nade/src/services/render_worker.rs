use core::{FrameData, SceneObjectData, TextObject};
use crossbeam_channel::{Receiver, Sender, TrySendError, bounded};
use std::thread;

/// 1フレーム分のレンダリング要求
pub struct RenderJob {
	pub objects: Vec<TextObject>,
	pub time: f32,
	pub width: u32,
	pub height: u32,
}

/// 常駐レンダリングワーカー
///
/// フレームごとに `thread::spawn` する代わりに、専用スレッドがジョブを処理する。
/// チャンネル容量 1 なので、処理中に届いた最新フレームだけを 1 件キューできる。
pub struct RenderWorker {
	job_tx: Sender<RenderJob>,
}

impl RenderWorker {
	pub fn start(render_tx: Sender<FrameData>) -> Self {
		let (job_tx, job_rx): (Sender<RenderJob>, Receiver<RenderJob>) = bounded(1);

		thread::Builder::new()
			.name("nade-render".into())
			.spawn(move || {
				while let Ok(job) = job_rx.recv() {
					let img_buffer = renderer::render_objects(
						job.objects.iter().map(|obj| obj as &dyn SceneObjectData),
						job.time,
						job.width,
						job.height,
					);
					let frame_data = FrameData {
						width: job.width,
						height: job.height,
						pixels: bytes::Bytes::from(img_buffer.into_raw()),
					};
					if render_tx.send(frame_data).is_err() {
						break;
					}
				}
			})
			.expect("failed to spawn render worker");

		Self { job_tx }
	}

	pub fn try_submit(&self, job: RenderJob) -> bool {
		match self.job_tx.try_send(job) {
			Ok(()) => true,
			Err(TrySendError::Full(_)) => false,
			Err(TrySendError::Disconnected(_)) => false,
		}
	}
}
