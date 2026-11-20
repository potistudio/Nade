//! # メッセージ定義
//!
//! アプリケーション全体で使用されるメッセージ型を定義します。

use nade_curve_editor_panel::CurveEditorMessage;
use nade_panel_system::PanelSystemMessage;
use nade_timeline_panel::TimelineMessage;
use std::path::PathBuf;

use crate::panel_content::PanelContent;
use crate::workspace::Workspace;
use nade_core::{FrameData, MediaInfo};

pub use nade_asset_browser::AssetBrowserMessage;
use nade_asset_browser::ImportTicket;
pub use nade_browser_panel::ProjectPaneMessage;
pub use nade_viewer_panel::{PreviewMessage, Tool, TransportMessage, ViewerRequest};

/// メニューバーとコンテキストメニュー
#[derive(Debug, Clone)]
pub enum MenuMessage {
	/// メニューバーのタイトルを押した（開閉）
	ToggleBar(crate::shell::menu::MenuKind),
	/// 開いている間にタイトルへ乗ったら切り替える
	HoverBar(crate::shell::menu::MenuKind),
	/// 右クリック位置（ウィンドウ座標）にコンテキストメニューを開く
	OpenContext(iced::Point),
	Close,
	/// 項目を選んだ: メニューを閉じてから実行する
	Run(Box<Message>),
	/// macOS のシステムメニューバーで項目が選ばれた（項目 ID）
	#[cfg(target_os = "macos")]
	Native(String),
}

/// アプリケーションパネルメッセージ（ラッパー）
#[derive(Debug, Clone)]
pub enum AppPanelMessage {
	Timeline(TimelineMessage),
	CurveEditor(CurveEditorMessage),
	Project(ProjectPaneMessage),
	Assets(AssetBrowserMessage),
	Inspector(nade_inspector_panel::InspectorMessage),
	Preview(PreviewMessage),
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
	/// ワークエリアをコンポ全体に戻す
	ClearWorkArea,
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
	/// フォーカス中のパネルの選択項目を名前変更する（Enter）
	Rename,
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
	/// Esc。パネルのドラッグ中ならその取り消し、そうでなければ書き出しの中止（`bool`: キーが未処理だったか）
	EscapePressed(bool),
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
	/// プローブ済みのメディアを選択中の取り込み先へ追加する（開発ツール用の同期経路）
	MediaImported(Vec<(PathBuf, Result<MediaInfo, String>)>),
	/// ファイルダイアログでメディアが選ばれた（キャンセル時は空）
	MediaPicked(Vec<PathBuf>),
	/// 取り込み中のファイルのプローブが終わった
	MediaProbed(ImportTicket, PathBuf, Result<MediaInfo, String>),
	/// ウィンドウへファイルがドロップされた（.nade は開き、他は読み込む）
	FileDropped(PathBuf),
	/// ファイルがウィンドウ上へドラッグされてきた
	FileHovered(PathBuf),
	/// ドラッグ中のファイルがウィンドウから離れた
	FilesHoveredLeft,
	Menu(MenuMessage),
	/// デバッグ用スクリーンショットの取得結果
	#[cfg(debug_assertions)]
	DevScreenshot(iced::window::Screenshot),
	/// デバッグ用: プローブせずに取り込み中の行だけを出す
	#[cfg(debug_assertions)]
	DevStageImport(PathBuf),
}
