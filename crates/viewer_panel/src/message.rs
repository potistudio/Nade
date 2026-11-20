//! Viewer panel messages.

use nade_core::InstanceId;

use crate::settings::Resolution;

/// プレビュー（ビューア）パネルメッセージ
#[derive(Debug, Clone)]
pub enum PreviewMessage {
	/// ビューポートのパン／ズームが変わった
	ViewChanged {
		zoom: f32,
		offset: [f32; 2],
	},
	/// ビューア領域のサイズが変わった（倍率表示用）
	ViewportSized(iced::Size),
	/// コンポジション全体が収まるように表示
	FitView,
	/// 表示倍率を掛ける（ビューア中心基準）
	ZoomBy(f32),
	/// 実寸 (100%) 表示
	ActualPixels,
	ToggleGrid,
	ToggleSafeAreas,
	SetResolution(Resolution),
	/// トランスポートバーの操作
	Transport(TransportMessage),
	/// ビューア上でレイヤーを選択（None で選択解除）
	SelectLayer(Option<InstanceId>),
	/// 選択レイヤーを移動（親空間＝position の座標系での差分）
	MoveLayer {
		delta: [f32; 2],
	},
	/// 選択レイヤーのスケールを設定（倍率、1.0 = 100%）
	ScaleLayer {
		scale: [f32; 2],
	},
	/// アンカーポイントだけを動かし、見た目は保つ（Pan Behind）
	MoveAnchor {
		/// レイヤー空間での差分
		anchor: [f32; 2],
		/// 親空間での position の差分
		position: [f32; 2],
	},
	/// 選択レイヤーを回転（度）
	RotateLayer {
		degrees: f32,
	},
	/// 文字ツールでテキストレイヤーを配置（コンポ中心基準の座標）
	CreateText {
		position: [f32; 2],
	},
}

/// 再生コントロール（ビューアのトランスポートとショートカット共通）
#[derive(Debug, Clone, Copy)]
pub enum TransportMessage {
	TogglePlay,
	GoToStart,
	GoToEnd,
	StepBackward,
	StepForward,
	ToggleLoop,
	/// I / O: 選択レイヤーのインポイント／アウトポイントへ
	GoToLayerIn,
	GoToLayerOut,
	/// J / K: 前／次のキーフレーム・レイヤー端・ワークエリア端へ
	PreviousEditPoint,
	NextEditPoint,
}
