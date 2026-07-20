//! # パネルコンテンツ定義
//!
//! パネルに表示するコンテンツの種類を定義します。

use panel_system::AreaKind;
use std::fmt;

/// パネルに表示するコンテンツの種類
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanelContent {
	/// メインプレビュー（ピクセルビューア）
	MainPreview,
	/// タイムライン
	Timeline,
	/// インスペクター（Transform/Graph）
	Inspector,
	/// プロジェクト（アセットブラウザ）
	Project,
}

impl fmt::Display for PanelContent {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::MainPreview => "Preview",
			Self::Timeline => "Timeline",
			Self::Inspector => "Inspector",
			Self::Project => "Project",
		})
	}
}

impl AreaKind for PanelContent {
	fn all() -> Vec<Self> {
		vec![
			Self::MainPreview,
			Self::Timeline,
			Self::Inspector,
			Self::Project,
		]
	}
}
