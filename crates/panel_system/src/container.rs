//! エリア（タブ付きのエディタ領域）

/// レイアウト上の1エリア。1つ以上のタブを持ち、アクティブなタブのみを表示する。
#[derive(Debug, Clone)]
pub struct Area<C: Clone + std::fmt::Debug> {
	pub id: usize,
	/// 常に1要素以上。空になったエリアはレイアウトから取り除く。
	/// 同じエディタは1エリアに1タブまで（[`PanelSystem`](crate::PanelSystem) が重複を作らない）。
	tabs: Vec<C>,
	active: usize,
}

impl<C: Clone + std::fmt::Debug> Area<C> {
	/// 単一タブのエリアを作成する。
	pub fn new(id: usize, content: C) -> Self {
		Self {
			id,
			tabs: vec![content],
			active: 0,
		}
	}

	/// 複数タブのエリアを作成する。`active` は範囲内に丸める。
	///
	/// # Panics
	///
	/// `tabs` が空の場合。
	pub fn with_tabs(id: usize, tabs: Vec<C>, active: usize) -> Self {
		assert!(!tabs.is_empty(), "an area needs at least one tab");
		let active = active.min(tabs.len() - 1);
		Self { id, tabs, active }
	}

	/// 全タブのコンテンツ（表示順）。
	pub fn tabs(&self) -> &[C] {
		&self.tabs
	}

	/// アクティブなタブの位置。
	pub fn active_index(&self) -> usize {
		self.active
	}

	/// アクティブなタブのコンテンツ。
	pub fn content(&self) -> &C {
		&self.tabs[self.active]
	}

	pub(crate) fn set_content(&mut self, content: C) {
		self.tabs[self.active] = content;
	}

	pub(crate) fn activate(&mut self, index: usize) {
		if index < self.tabs.len() {
			self.active = index;
		}
	}

	/// `index` の手前にタブを挿入し、アクティブにする。
	pub(crate) fn insert_tab(&mut self, index: usize, content: C) {
		let index = index.min(self.tabs.len());
		self.tabs.insert(index, content);
		self.active = index;
	}

	/// タブを取り除く。最後のタブを取り除くとエリアは空になる。
	///
	/// アクティブなタブを取り除いた場合、右隣（末尾なら左隣）がアクティブになる。
	pub(crate) fn remove_tab(&mut self, index: usize) -> Option<C> {
		if index >= self.tabs.len() {
			return None;
		}
		let content = self.tabs.remove(index);
		if index < self.active || self.active >= self.tabs.len() {
			self.active = self.active.saturating_sub(1);
		}
		Some(content)
	}

	/// `from` のタブを挿入位置 `slot`（`0..=len`、移動前の並びで数える）へ移し、アクティブにする。
	pub(crate) fn move_tab(&mut self, from: usize, slot: usize) {
		if from >= self.tabs.len() {
			return;
		}
		let content = self.tabs.remove(from);
		let to = if slot > from { slot - 1 } else { slot };
		self.insert_tab(to, content);
	}

	pub(crate) fn is_empty(&self) -> bool {
		self.tabs.is_empty()
	}
}
