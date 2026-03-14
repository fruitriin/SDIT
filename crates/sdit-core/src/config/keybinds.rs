//! キーバインド設定スキーマ。
//!
//! winit に依存しない純粋な設定型を定義する。
//! `Key` / `ModifiersState` を使う `resolve_action` は `sdit` クレートの `input.rs` に配置する。

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Action
// ---------------------------------------------------------------------------

/// キーバインドで起動できるアクション。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum Action {
    NewWindow,
    AddSession,
    CloseSession,
    DetachSession,
    SidebarToggle,
    Copy,
    Paste,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Search,
    SearchNext,
    SearchPrev,
    NextSession,
    PrevSession,
    /// アプリケーションを終了する。
    Quit,
    /// バージョン情報を表示する。
    About,
    /// 設定ファイルをエディタで開く。
    Preferences,
    /// 全テキストを選択する（将来実装）。
    SelectAll,
    /// 前のプロンプト（OSC 133 シェルインテグレーション）にジャンプする。
    PrevPrompt,
    /// 次のプロンプト（OSC 133 シェルインテグレーション）にジャンプする。
    NextPrompt,
    /// `QuickSelect` モードを起動する（画面上のパターンをキーボードでコピー）。
    QuickSelect,
    /// vi モード（コピーモード）のトグル。
    ToggleViMode,
    /// Secure Keyboard Entry（セキュアキーボード入力）をトグルする。macOS のみ有効。
    ToggleSecureInput,
    /// 次のテーマに切り替える（テーマをサイクル）。
    NextTheme,
    /// 前のテーマに切り替える（テーマをサイクル）。
    PreviousTheme,
    /// ウィンドウデコレーション（タイトルバー等）の表示をトグルする。
    ToggleDecorations,
    /// ウィンドウを常に最前面に表示するかどうかをトグルする。
    ToggleAlwaysOnTop,
    /// コマンドパレットの表示をトグルする。
    ToggleCommandPalette,
}

impl Action {
    /// 全アクションバリアントの名前とアクションのペアを返す。
    ///
    /// `ToggleCommandPalette` は除外する（コマンドパレット内から自身をトグルするのを防止）。
    pub fn all_with_names() -> Vec<(&'static str, Action)> {
        vec![
            ("NewWindow", Action::NewWindow),
            ("AddSession", Action::AddSession),
            ("CloseSession", Action::CloseSession),
            ("DetachSession", Action::DetachSession),
            ("SidebarToggle", Action::SidebarToggle),
            ("Copy", Action::Copy),
            ("Paste", Action::Paste),
            ("ZoomIn", Action::ZoomIn),
            ("ZoomOut", Action::ZoomOut),
            ("ZoomReset", Action::ZoomReset),
            ("Search", Action::Search),
            ("SearchNext", Action::SearchNext),
            ("SearchPrev", Action::SearchPrev),
            ("NextSession", Action::NextSession),
            ("PrevSession", Action::PrevSession),
            ("Quit", Action::Quit),
            ("About", Action::About),
            ("Preferences", Action::Preferences),
            ("SelectAll", Action::SelectAll),
            ("PrevPrompt", Action::PrevPrompt),
            ("NextPrompt", Action::NextPrompt),
            ("QuickSelect", Action::QuickSelect),
            ("ToggleViMode", Action::ToggleViMode),
            ("ToggleSecureInput", Action::ToggleSecureInput),
            ("NextTheme", Action::NextTheme),
            ("PreviousTheme", Action::PreviousTheme),
            ("ToggleDecorations", Action::ToggleDecorations),
            ("ToggleAlwaysOnTop", Action::ToggleAlwaysOnTop),
            // ToggleCommandPalette は除外
        ]
    }
}

// ---------------------------------------------------------------------------
// KeyBinding
// ---------------------------------------------------------------------------

/// 1 つのキーバインド定義。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeyBinding {
    /// キー名。例: `n`, `t`, `Tab`, `backslash`, `[`, `]`
    pub key: String,
    /// モディファイア。例: "super", "ctrl", "super|shift"。省略可（空文字 = なし）。
    #[serde(default)]
    pub mods: String,
    /// 起動するアクション。
    pub action: Action,
    /// パース済みモディファイアのビットフィールドキャッシュ。
    /// `validate()` で設定される。0 = 未初期化 or モディファイアなし。
    /// ビット: 0=SUPER, 1=CTRL, 2=SHIFT, 3=ALT
    #[serde(skip)]
    pub cached_mods_bits: u8,
}

// ---------------------------------------------------------------------------
// KeybindConfig
// ---------------------------------------------------------------------------

/// バインディング件数の上限。DoS 防止用。
const MAX_BINDINGS: usize = 512;
/// key / mods フィールドの最大長。
const MAX_FIELD_LEN: usize = 64;

/// キーバインド設定全体。TOML では配列 `[[keybinds]]` に相当する。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(transparent)]
pub struct KeybindConfig {
    pub bindings: Vec<KeyBinding>,
}

impl KeybindConfig {
    /// バリデーション: 件数上限・フィールド長チェック + mods キャッシュ構築。
    /// 不正なエントリは除外し、件数上限を超えた分は切り捨てる。
    pub fn validate(&mut self) {
        self.bindings.retain(|b| {
            if b.key.len() > MAX_FIELD_LEN {
                log::warn!("Keybind key too long ({}), skipping", b.key.len());
                return false;
            }
            if b.mods.len() > MAX_FIELD_LEN {
                log::warn!("Keybind mods too long ({}), skipping", b.mods.len());
                return false;
            }
            true
        });
        if self.bindings.len() > MAX_BINDINGS {
            log::warn!(
                "Too many keybindings ({}), truncating to {MAX_BINDINGS}",
                self.bindings.len()
            );
            self.bindings.truncate(MAX_BINDINGS);
        }
        // mods キャッシュ構築（M-3: 実行時の文字列パースを排除）
        for binding in &mut self.bindings {
            binding.cached_mods_bits = parse_mods_to_bits(&binding.mods);
        }
    }
}

impl Default for KeybindConfig {
    fn default() -> Self {
        Self { bindings: default_bindings() }
    }
}

// ---------------------------------------------------------------------------
// デフォルトバインディング
// ---------------------------------------------------------------------------

/// プラットフォームごとのデフォルトキーバインド一覧を返す。
pub fn default_bindings() -> Vec<KeyBinding> {
    #[cfg(target_os = "macos")]
    {
        macos_default_bindings()
    }
    #[cfg(not(target_os = "macos"))]
    {
        other_default_bindings()
    }
}

#[cfg(target_os = "macos")]
#[allow(clippy::enum_glob_use)]
fn macos_default_bindings() -> Vec<KeyBinding> {
    use Action::*;
    vec![
        // ウィンドウ・セッション管理
        bind("n", "super", NewWindow),
        bind("n", "super|shift", DetachSession),
        bind("t", "super", AddSession),
        bind("w", "super", CloseSession),
        bind("backslash", "super", SidebarToggle),
        // クリップボード
        bind("c", "super", Copy),
        bind("v", "super", Paste),
        bind("a", "super", SelectAll),
        // ズーム
        // "=" → Cmd+= (SUPER のみ)、"plus" → Cmd++ = Cmd+Shift+= (SUPER|SHIFT)
        bind("=", "super", ZoomIn),
        bind("plus", "super|shift", ZoomIn),
        // "-" → Cmd+- (SUPER のみ)
        bind("-", "super", ZoomOut),
        bind("0", "super", ZoomReset),
        // 検索
        bind("f", "super", Search),
        bind("g", "super", SearchNext),
        bind("g", "super|shift", SearchPrev),
        // セッション切替
        bind("Tab", "ctrl", NextSession),
        bind("Tab", "ctrl|shift", PrevSession),
        bind("]", "super|shift", NextSession),
        bind("[", "super|shift", PrevSession),
        // プロンプトジャンプ（OSC 133 シェルインテグレーション）
        bind("up", "super", PrevPrompt),
        bind("down", "super", NextPrompt),
        // QuickSelect
        bind("space", "super|shift", QuickSelect),
        // vi モード（コピーモード）
        bind("v", "super|shift", ToggleViMode),
        // アプリ
        bind("q", "super", Quit),
        bind(",", "super", Preferences),
    ]
}

#[cfg(not(target_os = "macos"))]
#[allow(clippy::enum_glob_use)]
fn other_default_bindings() -> Vec<KeyBinding> {
    use Action::*;
    vec![
        // ウィンドウ・セッション管理
        bind("n", "ctrl|shift", NewWindow),
        bind("t", "ctrl|shift", AddSession),
        bind("w", "ctrl|shift", CloseSession),
        bind("backslash", "ctrl", SidebarToggle),
        // クリップボード
        bind("c", "ctrl|shift", Copy),
        bind("v", "ctrl|shift", Paste),
        // 検索
        bind("f", "ctrl", Search),
        bind("g", "ctrl", SearchNext),
        bind("g", "ctrl|shift", SearchPrev),
        // セッション切替
        bind("Tab", "ctrl", NextSession),
        bind("Tab", "ctrl|shift", PrevSession),
    ]
}

/// `KeyBinding` 構築ヘルパー。
fn bind(key: &str, mods: &str, action: Action) -> KeyBinding {
    KeyBinding {
        key: key.to_owned(),
        mods: mods.to_owned(),
        action,
        cached_mods_bits: parse_mods_to_bits(mods),
    }
}

// ---------------------------------------------------------------------------
// モディファイアビットフィールド
// ---------------------------------------------------------------------------

/// ビット定義: SUPER=1, CTRL=2, SHIFT=4, ALT=8
pub const MOD_BIT_SUPER: u8 = 1;
pub const MOD_BIT_CTRL: u8 = 2;
pub const MOD_BIT_SHIFT: u8 = 4;
pub const MOD_BIT_ALT: u8 = 8;

/// mods 文字列をビットフィールドにパースする（ロード時の1回のみ呼ぶ）。
pub fn parse_mods_to_bits(s: &str) -> u8 {
    let mut bits = 0u8;
    for token in s.split('|') {
        match token.trim().to_ascii_lowercase().as_str() {
            "super" | "cmd" | "logo" => bits |= MOD_BIT_SUPER,
            "ctrl" | "control" => bits |= MOD_BIT_CTRL,
            "shift" => bits |= MOD_BIT_SHIFT,
            "alt" | "option" => bits |= MOD_BIT_ALT,
            _ => {}
        }
    }
    bits
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings_not_empty() {
        let bindings = default_bindings();
        assert!(!bindings.is_empty(), "default_bindings() は空であってはならない");
    }

    #[test]
    fn default_keybind_config() {
        let config = KeybindConfig::default();
        assert!(!config.bindings.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_has_required_actions() {
        use std::collections::HashSet;
        let bindings = default_bindings();
        let actions: HashSet<_> = bindings.iter().map(|b| b.action).collect();
        for required in [
            Action::NewWindow,
            Action::AddSession,
            Action::CloseSession,
            Action::SidebarToggle,
            Action::Copy,
            Action::Paste,
            Action::ZoomIn,
            Action::ZoomOut,
            Action::ZoomReset,
            Action::Search,
            Action::NextSession,
            Action::PrevSession,
            Action::Quit,
            Action::SelectAll,
            Action::Preferences,
        ] {
            assert!(
                actions.contains(&required),
                "デフォルトバインディングに {required:?} が含まれていない"
            );
        }
    }

    /// TOML の `[keybinds]` セクション経由でのデシリアライズをテストするためのラッパー型。
    #[derive(serde::Deserialize)]
    struct KeybindWrapper {
        keybinds: KeybindConfig,
    }

    #[test]
    fn deserialize_keybind_config_from_toml() {
        let toml_str = r#"
[[keybinds]]
key = "n"
mods = "super"
action = "NewWindow"

[[keybinds]]
key = "t"
mods = "super|shift"
action = "AddSession"
"#;
        let wrapper: KeybindWrapper = toml::from_str(toml_str).unwrap();
        let config = wrapper.keybinds;
        assert_eq!(config.bindings.len(), 2);
        assert_eq!(config.bindings[0].action, Action::NewWindow);
        assert_eq!(config.bindings[0].key, "n");
        assert_eq!(config.bindings[0].mods, "super");
        assert_eq!(config.bindings[1].action, Action::AddSession);
        assert_eq!(config.bindings[1].mods, "super|shift");
    }

    #[test]
    fn deserialize_action_case_insensitive() {
        let toml_str = r#"
[[keybinds]]
key = "w"
mods = "super"
action = "CloseSession"
"#;
        let wrapper: KeybindWrapper = toml::from_str(toml_str).unwrap();
        let config = wrapper.keybinds;
        assert_eq!(config.bindings[0].action, Action::CloseSession);
    }

    #[test]
    fn toggle_secure_input_action_deserializes() {
        let toml_str = r#"
[[keybinds]]
key = "k"
mods = "super|shift"
action = "ToggleSecureInput"
"#;
        let wrapper: KeybindWrapper = toml::from_str(toml_str).unwrap();
        let config = wrapper.keybinds;
        assert_eq!(config.bindings[0].action, Action::ToggleSecureInput);
    }

    #[test]
    fn deserialize_mods_default_empty() {
        let toml_str = r#"
[[keybinds]]
key = "Tab"
action = "NextSession"
"#;
        let wrapper: KeybindWrapper = toml::from_str(toml_str).unwrap();
        let config = wrapper.keybinds;
        assert_eq!(config.bindings[0].mods, "");
    }
}
