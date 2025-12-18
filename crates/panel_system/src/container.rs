//! タブコンテナの定義
//!
//! 複数のパネルをタブとして管理するコンテナを提供します。

/// タブコンテナ
#[derive(Debug, Clone)]
pub struct TabContainer<C: Clone + std::fmt::Debug> {
	pub id: usize,
	pub panels: Vec<Panel<C>>,
	pub active_tab: usize,
}

/// パネル
#[derive(Debug, Clone)]
pub struct Panel<C: Clone + std::fmt::Debug> {
	pub id: usize,
	pub title: String,
	pub content: C,
}

impl<C: Clone + std::fmt::Debug> Panel<C> {
	pub fn new(id: usize, title: &str, content: C) -> Self {
		Self {
			id,
			title: title.to_string(),
			content,
		}
	}
}

impl<C: Clone + std::fmt::Debug> TabContainer<C> {
	pub fn new(id: usize) -> Self {
		Self {
			id,
			panels: Vec::new(),
			active_tab: 0,
		}
	}

	pub fn add_panel(&mut self, panel: Panel<C>) {
		self.panels.push(panel);
		self.active_tab = self.panels.len() - 1;
	}

	pub fn remove_panel(&mut self, panel_id: usize) -> bool {
		if let Some(pos) = self.panels.iter().position(|p| p.id == panel_id) {
			self.panels.remove(pos);
			if self.active_tab >= self.panels.len() && !self.panels.is_empty() {
				self.active_tab = self.panels.len() - 1;
			}
			true
		} else {
			false
		}
	}

	pub fn is_empty(&self) -> bool {
		self.panels.is_empty()
	}

	pub fn get_active_panel(&self) -> Option<&Panel<C>> {
		self.panels.get(self.active_tab)
	}

	pub fn get_panel(&self, panel_id: usize) -> Option<&Panel<C>> {
		self.panels.iter().find(|p| p.id == panel_id)
	}
}
