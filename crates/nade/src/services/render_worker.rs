use crossbeam_channel::{Receiver, Sender, TrySendError, bounded};
use nade_core::FrameData;
use nade_renderer::{Compositor, Scene};
use std::thread;

/// 1フレーム分のレンダリング要求
pub struct RenderJob {
	pub scene: Scene,
}

/// 常駐レンダリングワーカー
///
/// フレームごとに `thread::spawn` する代わりに、専用スレッドがジョブを処理する。
/// チャンネル容量 1 なので、処理中に届いた最新フレームだけを 1 件キューできる。
/// デコード済みフッテージ（動画デコーダ・静止画）はスレッド内で使い回す。
pub struct RenderWorker {
	job_tx: Sender<RenderJob>,
}

impl RenderWorker {
	pub fn start(render_tx: Sender<FrameData>) -> Self {
		let (job_tx, job_rx): (Sender<RenderJob>, Receiver<RenderJob>) = bounded(1);

		thread::Builder::new()
			.name("nade-render".into())
			.spawn(move || {
				let mut compositor = Compositor::new();
				while let Ok(job) = job_rx.recv() {
					let image = compositor.render(&job.scene);
					let frame_data = FrameData {
						width: job.scene.width,
						height: job.scene.height,
						pixels: bytes::Bytes::from(image.into_raw()),
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
