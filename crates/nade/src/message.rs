//! # メッセージ定義
//!
//! アプリケーション全体で使用されるメッセージ型を定義します。

use panel_system::PanelSystemMessage;
use timeline_panel::TimelineMessage;

use crate::panel_content::PanelContent;
pub use inspector_panel::InspectorMessage;
use nade_core::FrameData;

pub use project_panel::ProjectPaneMessage;

/// アプリケーションパネルメッセージ（ラッパー）
#[derive(Debug, Clone)]
pub enum AppPanelMessage {
	Timeline {
		panel_id: usize,
		message: TimelineMessage,
	},
	Inspector(InspectorMessage),
	Project(ProjectPaneMessage),
}

/// アプリケーションメッセージ
#[derive(Debug, Clone)]
pub enum Message {
	/// 再生/一時停止の切り替え
	TogglePlay,
	/// 定期更新（再生中のアニメーション用）
	Tick,
	/// パネルシステムメッセージ
	PanelSystem(PanelSystemMessage<PanelContent, AppPanelMessage>),
	/// バックグラウンドレンダリング完了
	RenderCompleted(FrameData),
	/// ウィンドウが閉じられた
	WindowClosed(iced::window::Id),
}
