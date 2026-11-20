//! # メッセージ定義
//!
//! アプリケーション全体で使用されるメッセージ型を定義します。

use curve_editor_panel::CurveEditorMessage;
use panel_system::PanelSystemMessage;
use std::path::PathBuf;
use timeline_panel::TimelineMessage;

use crate::panel_content::PanelContent;
use crate::tool::Tool;
use crate::viewer::Resolution;
use crate::workspace::Workspace;
use core::{FrameData, InstanceId, MediaInfo};

pub use asset_browser::AssetBrowserMessage;
pub use browser_panel::ProjectPaneMessage;

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

/// アプリケーションパネルメッセージ（ラッパー）
#[derive(Debug, Clone)]
pub enum AppPanelMessage {
	Timeline(TimelineMessage),
	CurveEditor(CurveEditorMessage),
	Project(ProjectPaneMessage),
	Assets(AssetBrowserMessage),
	Inspector(inspector_panel::InspectorMessage),
	Preview(PreviewMessage),
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

/// 編集コマンド（メニュー・ショートカット共通）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditCommand {
	Undo,
	Redo,
	/// 再生ヘッド位置で選択レイヤー（無選択なら再生ヘッド下の全レイヤー）を分割
	Split,
	Duplicate,
	Copy,
	Cut,
	Paste,
	Delete,
	/// 選択レイヤーを新しいコンポジションへまとめる（AE の Pre-compose）
	Precompose,
	/// B / N: ワークエリアの開始／終了を再生ヘッドに
	WorkAreaStart,
	WorkAreaEnd,
	/// [ / ]: レイヤーのイン／アウトポイントが再生ヘッドに来るよう移動
	MoveInPoint,
	MoveOutPoint,
	/// Alt+[ / Alt+]: イン／アウトポイントを再生ヘッドでトリム
	TrimInPoint,
	TrimOutPoint,
	/// F9: 選択キー（無ければ再生ヘッド上のキー）にイージーイーズ
	EasyEase,
}

/// ウィンドウ上部のツールバー／タイトルストリップからの操作
#[derive(Debug, Clone, Copy)]
pub enum ShellMessage {
	SelectTool(Tool),
	SelectWorkspace(Workspace),
	NewComposition,
	NewTextLayer,
	NewSolid,
	ImportMedia,
	/// プロジェクトを開く
	OpenProject,
	/// プロジェクトを保存（未保存なら保存先を尋ねる）
	SaveProject,
	SaveProjectAs,
	/// 現在のコンポジションを動画ファイルへ書き出す
	Export,
	CancelExport,
	/// タイトルストリップのドラッグでウィンドウを移動
	DragWindow,
	/// タイトルストリップのダブルクリックで最大化を切り替え
	ToggleMaximize,
}

/// アプリケーションメッセージ
#[derive(Debug, Clone)]
pub enum Message {
	/// 再生コントロール
	Transport(TransportMessage),
	/// ツールバー操作
	Shell(ShellMessage),
	/// 編集コマンド
	Edit(EditCommand),
	/// マウスボタンが離された（ドラッグ操作の区切り。Undo 履歴の確定に使う）
	PointerReleased,
	/// ウィンドウサイズ変更
	WindowResized(iced::Size),
	/// 定期更新（再生中のアニメーション用）
	Tick,
	/// パネルシステムメッセージ
	PanelSystem(PanelSystemMessage<PanelContent, AppPanelMessage>),
	/// バックグラウンドレンダリング完了
	RenderCompleted(FrameData),
	/// ウィンドウの閉じる操作が要求された（アプリケーション終了）
	CloseRequested(iced::window::Id),
	/// タイムラインパネルメッセージ
	Timeline(TimelineMessage),
	/// 開くプロジェクトファイルが選ばれた（キャンセル時は None）
	ProjectChosen(Option<PathBuf>),
	/// プロジェクトの保存先が選ばれた（キャンセル時は None）
	SaveProjectTo(Option<PathBuf>),
	/// 書き出し先が選ばれた（キャンセル時は None）
	ExportTo(Option<PathBuf>),
	/// ファイルダイアログで選ばれ、プローブ済みのメディア（キャンセル時は空）
	MediaImported(Vec<(PathBuf, Result<MediaInfo, String>)>),
	/// ウィンドウへファイルがドロップされた（.nade は開き、他は読み込む）
	FileDropped(PathBuf),
	/// デバッグ用スクリーンショットの取得結果
	#[cfg(debug_assertions)]
	DevScreenshot(iced::window::Screenshot),
}
