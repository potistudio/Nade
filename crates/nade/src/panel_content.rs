//! # パネルコンテンツ定義
//!
//! パネルに表示するコンテンツの種類を定義します。

use iced::Color;

/// パネルに表示するコンテンツの種類
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanelContent {
	/// メインプレビュー（ピクセルビューア）
	MainPreview,
	/// タイムライン
	Timeline,
	/// プロパティ（パラメータ編集）
	Properties,
	/// プロジェクト（アセットブラウザ）
	Project,
	/// コンソール（ログ出力）
	Console,
}

impl PanelContent {
	/// パネルの色を返す（識別用）
	pub fn color(&self) -> Color {
		match self {
			PanelContent::MainPreview => Color::from_rgb(0.2, 0.3, 0.4),
			PanelContent::Timeline => Color::from_rgb(0.3, 0.2, 0.4),
			PanelContent::Properties => Color::from_rgb(0.2, 0.4, 0.3),
			PanelContent::Project => Color::from_rgb(0.4, 0.3, 0.2),
			PanelContent::Console => Color::from_rgb(0.15, 0.15, 0.15),
		}
	}
}
