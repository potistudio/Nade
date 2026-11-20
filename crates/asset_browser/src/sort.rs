//! Sibling ordering of the asset browser outline.
//!
//! Sorting only reorders siblings for display; the project tree keeps its own
//! (import) order untouched.

use nade_domain::{Asset, AssetType};
use std::cmp::Ordering;

/// Column the outline is ordered by.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortKey {
	#[default]
	Name,
	Kind,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortDirection {
	#[default]
	Ascending,
	Descending,
}

impl SortDirection {
	pub fn flipped(self) -> Self {
		match self {
			Self::Ascending => Self::Descending,
			Self::Descending => Self::Ascending,
		}
	}
}

/// Active sort column and direction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SortOrder {
	pub key: SortKey,
	pub direction: SortDirection,
}

// -------- Public API ---------------------------------------------------------
impl SortOrder {
	/// Header click: the active column flips direction, another column starts
	/// ascending.
	pub fn toggled(self, key: SortKey) -> Self {
		if self.key == key {
			Self {
				key,
				direction: self.direction.flipped(),
			}
		} else {
			Self {
				key,
				direction: SortDirection::Ascending,
			}
		}
	}

	/// Sorts siblings in place. Folders always stay on top, whatever the
	/// column or direction.
	pub fn sort(self, siblings: &mut [&Asset]) {
		// 名前比較用のキーは要素ごとに一度だけ作る
		siblings.sort_by_cached_key(|asset| SortEntry::new(asset, self));
	}
}

// -------- Internal -----------------------------------------------------------
/// Precomputed comparison key of one sibling.
struct SortEntry {
	is_folder: bool,
	order: SortOrder,
	kind_rank: u8,
	name: String,
}

impl SortEntry {
	fn new(asset: &Asset, order: SortOrder) -> Self {
		Self {
			is_folder: asset.kind() == AssetType::Folder,
			order,
			kind_rank: kind_rank(asset.kind()),
			name: asset.name.to_lowercase(),
		}
	}
}

impl Ord for SortEntry {
	fn cmp(&self, other: &Self) -> Ordering {
		let directed = |ordering: Ordering| match self.order.direction {
			SortDirection::Ascending => ordering,
			SortDirection::Descending => ordering.reverse(),
		};
		let by_name = || natural_cmp(&self.name, &other.name);
		let primary = match self.order.key {
			SortKey::Name => directed(by_name()),
			// 同じ種類の中では名前の昇順で安定させる
			SortKey::Kind => directed(self.kind_rank.cmp(&other.kind_rank)).then_with(by_name),
		};
		other.is_folder.cmp(&self.is_folder).then(primary)
	}
}

impl PartialOrd for SortEntry {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl PartialEq for SortEntry {
	fn eq(&self, other: &Self) -> bool {
		self.cmp(other) == Ordering::Equal
	}
}

impl Eq for SortEntry {}

/// Order of the kind column, matching the row's glyph groups.
fn kind_rank(kind: AssetType) -> u8 {
	match kind {
		AssetType::Folder => 0,
		AssetType::Composition => 1,
		AssetType::Video => 2,
		AssetType::Image => 3,
		AssetType::Audio => 4,
	}
}

/// Compares names with digit runs as numbers, so `clip2` sorts before
/// `clip10`.
fn natural_cmp(a: &str, b: &str) -> Ordering {
	let mut a = a.chars().peekable();
	let mut b = b.chars().peekable();
	loop {
		match (a.peek().copied(), b.peek().copied()) {
			(None, None) => return Ordering::Equal,
			(None, Some(_)) => return Ordering::Less,
			(Some(_), None) => return Ordering::Greater,
			(Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
				let x = take_number(&mut a);
				let y = take_number(&mut b);
				// 先頭ゼロを除いた桁数 → 桁ごとの順で比べれば桁あふれしない
				let ordering = x.len().cmp(&y.len()).then_with(|| x.cmp(&y));
				if ordering != Ordering::Equal {
					return ordering;
				}
			}
			(Some(x), Some(y)) => {
				if x != y {
					return x.cmp(&y);
				}
				a.next();
				b.next();
			}
		}
	}
}

/// Consumes one digit run and returns it without leading zeros.
fn take_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
	let mut digits = String::new();
	while let Some(c) = chars.next_if(char::is_ascii_digit) {
		if !(digits.is_empty() && c == '0') {
			digits.push(c);
		}
	}
	digits
}

#[cfg(test)]
mod tests {
	use super::*;
	use nade_core::AssetId;

	fn asset(name: &str, kind: AssetType) -> Asset {
		Asset::new(AssetId::new(0), name.into(), kind, None)
	}

	fn sorted(order: SortOrder, assets: &[Asset]) -> Vec<&str> {
		let mut siblings: Vec<&Asset> = assets.iter().collect();
		order.sort(&mut siblings);
		siblings.iter().map(|asset| asset.name.as_str()).collect()
	}

	/// Digit runs compare by value, letters compare case-insensitively.
	#[test]
	fn names_sort_naturally() {
		let assets = [
			asset("clip10.mov", AssetType::Video),
			asset("Clip2.mov", AssetType::Video),
			asset("clip02b.mov", AssetType::Video),
			asset("b-roll.mov", AssetType::Video),
		];

		assert_eq!(
			sorted(SortOrder::default(), &assets),
			["b-roll.mov", "Clip2.mov", "clip02b.mov", "clip10.mov"]
		);
	}

	/// Folders lead in both directions.
	#[test]
	fn folders_stay_on_top() {
		let assets = [
			asset("a.wav", AssetType::Audio),
			asset("Zeta", AssetType::Folder),
			asset("z.png", AssetType::Image),
			asset("Alpha", AssetType::Folder),
		];
		let descending = SortOrder::default().toggled(SortKey::Name);

		assert_eq!(
			sorted(SortOrder::default(), &assets),
			["Alpha", "Zeta", "a.wav", "z.png"]
		);
		assert_eq!(sorted(descending, &assets), ["Zeta", "Alpha", "z.png", "a.wav"]);
	}

	/// Kind groups follow the column direction; names inside a group stay
	/// ascending.
	#[test]
	fn kind_sort_groups_by_kind_then_name() {
		let assets = [
			asset("b.wav", AssetType::Audio),
			asset("a.wav", AssetType::Audio),
			asset("clip.mov", AssetType::Video),
			asset("Main", AssetType::Composition),
		];
		let ascending = SortOrder::default().toggled(SortKey::Kind);

		assert_eq!(sorted(ascending, &assets), ["Main", "clip.mov", "a.wav", "b.wav"]);
		assert_eq!(
			sorted(ascending.toggled(SortKey::Kind), &assets),
			["a.wav", "b.wav", "clip.mov", "Main"]
		);
	}

	/// Re-clicking the active column flips it; a new column starts ascending.
	#[test]
	fn toggled_flips_or_switches_column() {
		let order = SortOrder::default();

		let flipped = order.toggled(SortKey::Name);
		assert_eq!(flipped.direction, SortDirection::Descending);

		let switched = flipped.toggled(SortKey::Kind);
		assert_eq!(
			switched,
			SortOrder {
				key: SortKey::Kind,
				direction: SortDirection::Ascending,
			}
		);
	}
}
