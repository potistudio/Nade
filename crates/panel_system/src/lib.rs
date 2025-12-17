//! # Panel System
//!
//! Icedアプリケーション用のドッカブルパネルシステム。
//! タブ化されたパネル、分割レイアウト、ドラッグ＆ドロップによるパネル移動をサポートします。

#![allow(dead_code)]

mod consts;
mod container;
mod drag;
mod node;
mod system;

pub use container::{Panel, TabContainer};
pub use drag::{DragState, DropPosition, DropZone};
pub use node::{DockNode, SplitDirection};
pub use system::{LayoutBuilder, PanelSystem, PanelSystemMessage};

use iced::Element;

/// パネルコンテンツを提供するトレイト
///
/// アプリケーション側でこのトレイトを実装することで、
/// 各パネルの内容をカスタマイズできます。
pub trait PanelContentProvider {
	/// パネルのコンテンツタイプ
	type Content: Clone + std::fmt::Debug + PartialEq + Eq;

	/// メッセージタイプ
	type Message: Clone + std::fmt::Debug;

	/// パネルコンテンツを描画
	fn view_content<'a>(
		&'a self,
		content: &Self::Content,
	) -> Element<'a, PanelSystemMessage<Self::Content>>;

	/// パネルのタイトルを取得
	fn title(content: &Self::Content) -> &'static str;

	/// パネルのアイコンを取得
	fn icon(content: &Self::Content) -> &'static str;
}
