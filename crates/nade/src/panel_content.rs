//! # パネルコンテンツ定義
//!
//! パネルに表示するコンテンツの種類を定義します。

use iced::widget::svg;
use panel_system::AreaKind;

/// パネルに表示するコンテンツの種類
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanelContent {
	/// メインプレビュー（ピクセルビューア）
	MainPreview,
	/// タイムライン
	Timeline,
	/// アニメーションカーブエディター
	CurveEditor,
	/// インスペクター（Transform/Graph）
	Inspector,
	/// オブジェクトブラウザ（Composition → Object アウトライン）
	Project,
	/// アセットブラウザ（メディア / フォルダビン）
	Assets,
}

impl AreaKind for PanelContent {
	fn all() -> Vec<Self> {
		vec![
			Self::MainPreview,
			Self::Timeline,
			Self::CurveEditor,
			Self::Inspector,
			Self::Project,
			Self::Assets,
		]
	}

	fn icon(&self) -> svg::Handle {
		let bytes: &'static [u8] = match self {
			Self::MainPreview => include_bytes!("../../../assets/icons/panels/preview.svg"),
			Self::Timeline => include_bytes!("../../../assets/icons/panels/timeline.svg"),
			Self::CurveEditor => include_bytes!("../../../assets/icons/panels/timeline.svg"),
			Self::Inspector => include_bytes!("../../../assets/icons/panels/inspector.svg"),
			Self::Project => include_bytes!("../../../assets/icons/panels/project.svg"),
			Self::Assets => include_bytes!("../../../assets/icons/panels/project.svg"),
		};
		svg::Handle::from_memory(bytes)
	}

	fn label(&self) -> &'static str {
		match self {
			Self::MainPreview => "Preview",
			Self::Timeline => "Timeline",
			Self::CurveEditor => "Curve Editor",
			Self::Inspector => "Inspector",
			Self::Project => "Objects",
			Self::Assets => "Assets",
		}
	}

	fn menu_columns() -> Vec<(&'static str, Vec<Self>)> {
		vec![
			("General", vec![Self::MainPreview]),
			("Animation", vec![Self::Timeline, Self::CurveEditor]),
			("Data", vec![Self::Project, Self::Assets, Self::Inspector]),
		]
	}
}
