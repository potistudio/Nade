//! # メッセージ定義
//!
//! アプリケーション全体で使用されるメッセージ型を定義します。

use panel_system::PanelSystemMessage;
use std::path::PathBuf;
use timeline_panel::TimelineMessage;

use crate::panel_content::PanelContent;
use core::FrameData;

pub use asset_browser::AssetBrowserMessage;
pub use browser_panel::ProjectPaneMessage;

/// プレビューパネルメッセージ
#[derive(Debug, Clone)]
pub enum PreviewMessage {
	/// ビューポートのパン／ズームが変わった
	ViewChanged { zoom: f32, offset: [f32; 2] },
}

/// アプリケーションパネルメッセージ（ラッパー）
#[derive(Debug, Clone)]
pub enum AppPanelMessage {
	Timeline { panel_id: usize, message: TimelineMessage },
	Project(ProjectPaneMessage),
	Assets(AssetBrowserMessage),
	Inspector(inspector_panel::InspectorMessage),
	Preview(PreviewMessage),
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
	/// タイムラインパネルメッセージ
	Timeline(TimelineMessage),
	/// ファイルダイアログで選ばれたメディア（キャンセル時は None）
	MediaImported(Option<PathBuf>),
}
