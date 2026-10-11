//! # パネルコンテンツ定義
//!
//! パネルに表示するコンテンツの種類を定義します。

use constants::icons::Icon;
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
		let icon = match self {
			Self::MainPreview => Icon::PanelViewer,
			Self::Timeline => Icon::PanelTimeline,
			Self::CurveEditor => Icon::PanelCurves,
			Self::Inspector => Icon::PanelInspector,
			Self::Project => Icon::PanelLayers,
			Self::Assets => Icon::PanelAssets,
		};
		icon.handle()
	}

	fn label(&self) -> &'static str {
		match self {
			Self::MainPreview => "Viewer",
			Self::Timeline => "Timeline",
			Self::CurveEditor => "Curve Editor",
			Self::Inspector => "Properties",
			Self::Project => "Layers",
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
