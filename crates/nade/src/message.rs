//! # メッセージ定義
//!
//! アプリケーション全体で使用されるメッセージ型を定義します。

use panel_system::PanelSystemMessage;
use timeline_pane::TimelineMessage;

use crate::panel_content::PanelContent;
use nade_core::FrameData;

/// アプリケーションパネルメッセージ（ラッパー）
#[derive(Debug, Clone)]
pub enum AppPanelMessage {
	Timeline(TimelineMessage),
	Property(PropertyMessage),
	Project(ProjectMessage),
}

/// プロジェクトパネルメッセージ
#[derive(Debug, Clone)]
pub enum ProjectMessage {
	ToggleExpand(uuid::Uuid),
	Select(uuid::Uuid),
}

/// プロパティパネルメッセージ
#[derive(Debug, Clone)]
pub enum PropertyMessage {
	Position(usize, f32), // axis (0=x, 1=y, 2=z), value
	Rotation(usize, f32),
	Scale(usize, f32),
	Opacity(f32),
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
