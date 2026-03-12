/// サイドバーの論理状態（GUI に依存しない）。
///
/// セッションが2つ以上のとき自動表示、1つのとき非表示。
/// `Cmd+\` で手動トグルも可能。
pub struct SidebarState {
    /// サイドバーの幅（セル数）。
    pub width_cells: usize,
    /// サイドバーが表示されているか。
    pub visible: bool,
    /// ユーザーが手動でトグルしたか（自動制御を上書き）。
    manual_override: bool,
}

impl SidebarState {
    pub fn new() -> Self {
        Self { width_cells: 20, visible: false, manual_override: false }
    }

    /// セッション数に基づいて表示状態を自動更新する。
    ///
    /// 手動オーバーライドされている場合は、セッション数が1になったときだけ
    /// 強制的に非表示にする。
    pub fn auto_update(&mut self, session_count: usize) {
        if session_count <= 1 {
            self.visible = false;
            self.manual_override = false;
        } else if !self.manual_override {
            self.visible = true;
        }
    }

    /// 手動でサイドバーの表示/非表示をトグルする。
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        self.manual_override = true;
    }

    /// Y 座標からセッションインデックスを返す（ヒットテスト）。
    ///
    /// サイドバーが非表示なら常に `None` を返す。
    pub fn hit_test(&self, y_px: f32, cell_height: f32, session_count: usize) -> Option<usize> {
        if !self.visible || cell_height <= 0.0 || y_px < 0.0 {
            return None;
        }
        let row = (y_px / cell_height) as usize;
        if row < session_count { Some(row) } else { None }
    }

    /// サイドバーの幅をピクセルで返す。非表示なら 0。
    pub fn width_px(&self, cell_width: f32) -> f32 {
        if self.visible { self.width_cells as f32 * cell_width } else { 0.0 }
    }
}

impl Default for SidebarState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_update_single_session() {
        let mut state = SidebarState::new();
        state.auto_update(1);
        assert!(!state.visible);
    }

    #[test]
    fn test_auto_update_multiple_sessions() {
        let mut state = SidebarState::new();
        state.auto_update(2);
        assert!(state.visible);
    }

    #[test]
    fn test_auto_update_back_to_single() {
        let mut state = SidebarState::new();
        state.auto_update(3);
        assert!(state.visible);
        state.auto_update(1);
        assert!(!state.visible);
    }

    #[test]
    fn test_manual_toggle() {
        let mut state = SidebarState::new();
        state.auto_update(2);
        assert!(state.visible);
        state.toggle();
        assert!(!state.visible);
        // 手動オーバーライド中は auto_update で上書きされない
        state.auto_update(2);
        assert!(!state.visible);
    }

    #[test]
    fn test_manual_override_cleared_on_single() {
        let mut state = SidebarState::new();
        state.auto_update(2);
        state.toggle(); // 手動で非表示に
        assert!(!state.visible);
        state.auto_update(1); // 1セッション → 手動オーバーライド解除
        state.auto_update(2); // 再び2セッション → 自動表示
        assert!(state.visible);
    }

    #[test]
    fn test_hit_test_visible() {
        let state = SidebarState { width_cells: 20, visible: true, manual_override: false };
        assert_eq!(state.hit_test(0.0, 20.0, 3), Some(0));
        assert_eq!(state.hit_test(25.0, 20.0, 3), Some(1));
        assert_eq!(state.hit_test(45.0, 20.0, 3), Some(2));
        assert_eq!(state.hit_test(65.0, 20.0, 3), None);
    }

    #[test]
    fn test_hit_test_invisible() {
        let state = SidebarState::new();
        assert_eq!(state.hit_test(10.0, 20.0, 3), None);
    }

    #[test]
    fn test_hit_test_negative_y() {
        let state = SidebarState { width_cells: 20, visible: true, manual_override: false };
        assert_eq!(state.hit_test(-5.0, 20.0, 3), None);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_width_px() {
        let mut state = SidebarState::new();
        assert_eq!(state.width_px(10.0), 0.0);
        state.auto_update(2);
        assert_eq!(state.width_px(10.0), 200.0);
    }
}
