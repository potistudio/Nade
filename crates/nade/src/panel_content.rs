//! # パネルコンテンツ定義
//!
//! パネルに表示するコンテンツの種類を定義します。

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
}
