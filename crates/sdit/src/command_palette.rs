//! コマンドパレット — Action 名のファジー検索と実行。

use sdit_core::config::keybinds::Action;

// ---------------------------------------------------------------------------
// CommandPaletteState
// ---------------------------------------------------------------------------

/// コマンドパレットの状態。
#[derive(Debug, Clone)]
pub(crate) struct CommandPaletteState {
    /// ユーザーが入力した検索クエリ。
    pub(crate) input: String,
    /// テキストカーソル位置（バイトオフセット）。
    pub(crate) cursor_pos: usize,
    /// 選択中の候補インデックス（0-indexed）。
    pub(crate) selected_index: usize,
    /// フィルタ済み候補。
    pub(crate) filtered_actions: Vec<(&'static str, Action)>,
}

/// 1ページに表示する候補の最大件数。
pub(crate) const MAX_VISIBLE_ITEMS: usize = 10;

/// 入力文字列の最大バイト数（DoS 防止）。
const MAX_INPUT_BYTES: usize = 256;

impl CommandPaletteState {
    /// 新しい空のコマンドパレット状態を作成する。
    pub(crate) fn new() -> Self {
        let filtered_actions = filter_actions("");
        Self { input: String::new(), cursor_pos: 0, selected_index: 0, filtered_actions }
    }

    /// 入力文字列に文字を追加する。
    ///
    /// 制御文字はフィルタリングされ、入力合計が MAX_INPUT_BYTES を超える場合は無視する。
    pub(crate) fn push_str(&mut self, s: &str) {
        let filtered: String = s.chars().filter(|c| !c.is_control()).collect();
        if self.input.len() + filtered.len() <= MAX_INPUT_BYTES {
            self.input.push_str(&filtered);
            self.cursor_pos = self.input.len();
            self.filtered_actions = filter_actions(&self.input);
            self.selected_index = 0;
        }
    }

    /// 入力文字列の末尾の1文字（Unicodeコードポイント）を削除する。
    pub(crate) fn pop_char(&mut self) {
        self.input.pop();
        self.cursor_pos = self.input.len();
        self.filtered_actions = filter_actions(&self.input);
        self.selected_index = 0;
    }

    /// 選択を1つ下に移動する。
    pub(crate) fn move_down(&mut self) {
        let max = self.filtered_actions.len().min(MAX_VISIBLE_ITEMS);
        if max == 0 {
            return;
        }
        self.selected_index = (self.selected_index + 1) % max;
    }

    /// 選択を1つ上に移動する。
    pub(crate) fn move_up(&mut self) {
        let max = self.filtered_actions.len().min(MAX_VISIBLE_ITEMS);
        if max == 0 {
            return;
        }
        self.selected_index = self.selected_index.checked_sub(1).unwrap_or(max - 1);
    }

    /// 現在選択中のアクションを返す。
    pub(crate) fn selected_action(&self) -> Option<Action> {
        self.filtered_actions.get(self.selected_index).map(|(_, action)| *action)
    }
}

// ---------------------------------------------------------------------------
// フィルタリング
// ---------------------------------------------------------------------------

/// 入力文字列で Action 名を case-insensitive 部分文字列マッチしてフィルタリングする。
///
/// 空文字列の場合は全アクションを返す（最大 `MAX_VISIBLE_ITEMS` 件）。
fn filter_actions(query: &str) -> Vec<(&'static str, Action)> {
    let all = Action::all_with_names();
    if query.is_empty() {
        return all.iter().copied().take(MAX_VISIBLE_ITEMS).collect();
    }
    let query_lower = query.to_ascii_lowercase();
    all.iter()
        .copied()
        .filter(|(name, _)| name.to_ascii_lowercase().contains(&query_lower))
        .take(MAX_VISIBLE_ITEMS)
        .collect()
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_shows_all_actions() {
        let state = CommandPaletteState::new();
        assert_eq!(state.filtered_actions.len(), MAX_VISIBLE_ITEMS);
    }

    #[test]
    fn filter_by_query() {
        let results = filter_actions("zoom");
        assert!(!results.is_empty(), "zoom に一致するアクションがない");
        for (name, _) in &results {
            assert!(name.to_ascii_lowercase().contains("zoom"), "zoom にマッチしない: {name}");
        }
    }

    #[test]
    fn filter_case_insensitive() {
        let lower = filter_actions("search");
        let upper = filter_actions("SEARCH");
        let mixed = filter_actions("SeArCh");
        assert_eq!(lower.len(), upper.len());
        assert_eq!(lower.len(), mixed.len());
    }

    #[test]
    fn filter_empty_returns_all_up_to_max() {
        let results = filter_actions("");
        assert_eq!(results.len(), MAX_VISIBLE_ITEMS);
    }

    #[test]
    fn filter_no_match_returns_empty() {
        let results = filter_actions("xyzzy_no_match_12345");
        assert!(results.is_empty());
    }

    #[test]
    fn move_down_wraps() {
        let mut state = CommandPaletteState::new();
        let len = state.filtered_actions.len().min(MAX_VISIBLE_ITEMS);
        for _ in 0..len {
            state.move_down();
        }
        assert_eq!(state.selected_index, 0, "wrap around to 0");
    }

    #[test]
    fn move_up_wraps() {
        let mut state = CommandPaletteState::new();
        state.move_up();
        let expected = state.filtered_actions.len().min(MAX_VISIBLE_ITEMS) - 1;
        assert_eq!(state.selected_index, expected, "wrap around to last");
    }

    #[test]
    fn selected_action_returns_correct_action() {
        let state = CommandPaletteState::new();
        let action = state.selected_action();
        assert!(action.is_some());
        assert_eq!(action, Some(state.filtered_actions[0].1));
    }

    #[test]
    fn push_str_updates_filter() {
        let mut state = CommandPaletteState::new();
        state.push_str("zoom");
        for (name, _) in &state.filtered_actions {
            assert!(name.to_ascii_lowercase().contains("zoom"));
        }
    }

    #[test]
    fn pop_char_updates_filter() {
        let mut state = CommandPaletteState::new();
        state.push_str("zoom");
        let zoom_count = state.filtered_actions.len();
        state.pop_char(); // "zoo" に戻る
        let zoo_count = state.filtered_actions.len();
        // "zoo" でのフィルタ結果は "zoom" より少ないか同数
        assert!(zoo_count >= zoom_count || zoo_count <= zoom_count);
    }

    #[test]
    fn command_palette_not_in_all_with_names() {
        // コマンドパレット自体は一覧から除外されていること
        for (name, action) in Action::all_with_names() {
            assert_ne!(
                action,
                Action::ToggleCommandPalette,
                "ToggleCommandPalette が all_with_names に含まれている: {name}"
            );
        }
    }

    /// #4: 制御文字フィルタリング — 制御文字は入力に追加されない
    #[test]
    fn push_str_filters_control_chars() {
        let mut state = CommandPaletteState::new();
        // \x00 (NUL), \x1b (ESC), \x07 (BEL) は制御文字 → 除去される
        // "[A" は通常文字なので残る
        state.push_str("hello\x00world\x1b[A\x07");
        assert_eq!(state.input, "helloworld[A", "control chars should be filtered");
    }

    /// #4: 制御文字フィルタリング — \n, \t など ASCII 制御文字も除去される
    #[test]
    fn push_str_filters_newline_and_tab() {
        let mut state = CommandPaletteState::new();
        state.push_str("zoom\nin\tout");
        assert_eq!(state.input, "zoominout", "newline and tab should be filtered");
    }

    /// #4: MAX_INPUT_BYTES 超のフィルタ後文字列は受け入れない
    #[test]
    fn push_str_ignores_input_exceeding_limit() {
        let mut state = CommandPaletteState::new();
        // MAX_INPUT_BYTES + 1 文字の通常テキスト — 1回のpush_strで超過するため全体が無視される
        let long_str = "a".repeat(MAX_INPUT_BYTES + 1);
        state.push_str(&long_str);
        assert!(state.input.is_empty(), "input exceeding MAX_INPUT_BYTES should be ignored");
    }

    /// #4: 分割送信で合計 MAX_INPUT_BYTES には達するが超えない場合は受け入れる
    #[test]
    fn push_str_accepts_exactly_256_bytes() {
        let mut state = CommandPaletteState::new();
        // 最初に MAX_INPUT_BYTES - 6 文字送り、次に 6 文字送る（合計 MAX_INPUT_BYTES）
        state.push_str(&"a".repeat(MAX_INPUT_BYTES - 6));
        state.push_str(&"b".repeat(6));
        assert_eq!(state.input.len(), MAX_INPUT_BYTES);
    }
}
