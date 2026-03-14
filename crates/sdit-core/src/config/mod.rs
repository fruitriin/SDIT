pub mod color;
pub mod font;
pub mod keybinds;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use self::color::ColorConfig;
use self::font::FontConfig;
use self::keybinds::KeybindConfig;
use crate::terminal::CursorStyle;

/// 起動時のウィンドウ表示モード。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum StartupMode {
    /// 通常ウィンドウ（デフォルト）。
    #[default]
    Windowed,
    /// 最大化ウィンドウ。
    Maximized,
    /// フルスクリーン（ボーダーレス）。
    Fullscreen,
}

/// カーソルスタイルの設定値（serde 用）。
///
/// `CursorStyle` とは別に定義し、TOML 文字列との相互変換を担う。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorStyleConfig {
    /// ブロックカーソル（デフォルト）。
    #[default]
    Block,
    /// アンダーラインカーソル。
    Underline,
    /// バーカーソル。
    Bar,
}

impl From<CursorStyleConfig> for CursorStyle {
    fn from(c: CursorStyleConfig) -> Self {
        match c {
            CursorStyleConfig::Block => CursorStyle::Block,
            CursorStyleConfig::Underline => CursorStyle::Underline,
            CursorStyleConfig::Bar => CursorStyle::Bar,
        }
    }
}

/// カーソル設定。
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct CursorConfig {
    /// カーソルスタイル: "block" (デフォルト), "underline", "bar"
    pub style: CursorStyleConfig,
    /// カーソル点滅を有効にする（デフォルト: false）。
    pub blinking: bool,
    /// カーソル色（hex 文字列、例: "#ff6600"）。None ならテーマ前景色を使用。
    pub color: Option<String>,
}

/// macOS の Option キーを Alt として扱うかどうかの設定。
///
/// readline ショートカット（Alt+B/F/D 等）を使用するのに必要。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionAsAlt {
    /// 左 Option キーのみ Alt として扱う。
    #[serde(alias = "left")]
    OnlyLeft,
    /// 右 Option キーのみ Alt として扱う。
    #[serde(alias = "right")]
    OnlyRight,
    /// 両方の Option キーを Alt として扱う。
    Both,
    /// Option キーを通常通り扱う（デフォルト）。
    #[serde(alias = "none")]
    None,
}

impl Default for OptionAsAlt {
    fn default() -> Self {
        Self::None
    }
}

/// セッション/ウィンドウを閉じるときの確認ダイアログ設定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmClose {
    /// 確認ダイアログを表示しない。
    Never,
    /// 常に確認ダイアログを表示する。
    Always,
    /// フォアグラウンドプロセスが実行中の場合のみ確認ダイアログを表示する（デフォルト）。
    ProcessRunning,
}

impl Default for ConfirmClose {
    fn default() -> Self {
        Self::ProcessRunning
    }
}

/// ウィンドウ外観の設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct WindowConfig {
    /// ウィンドウ背景の不透明度（0.0 = 完全透明、1.0 = 不透明）。
    pub opacity: f32,
    /// macOS でウィンドウ背景にブラーエフェクトを適用する。
    pub blur: bool,
    /// グリッドとウィンドウ左右端の余白（ピクセル）。デフォルト: 0、最大: 200。
    pub padding_x: u16,
    /// グリッドとウィンドウ上下端の余白（ピクセル）。デフォルト: 0、最大: 200。
    pub padding_y: u16,
    /// 初期ウィンドウ幅（列数）。デフォルト: 80、範囲: 10-500。
    pub columns: u16,
    /// 初期ウィンドウ高さ（行数）。デフォルト: 24、範囲: 2-200。
    pub rows: u16,
    /// 起動時のウィンドウ表示モード: "Windowed" (デフォルト), "Maximized", "Fullscreen"。
    pub startup_mode: StartupMode,
    /// 新しいセッション/ウィンドウ生成時に、アクティブセッションの作業ディレクトリを継承する（デフォルト: true）。
    pub inherit_working_directory: bool,
    /// セッションを閉じるときの確認ダイアログ設定。
    ///
    /// `"never"`: 確認なしで閉じる。
    /// `"always"`: 常に確認ダイアログを表示する。
    /// `"process_running"`: フォアグラウンドプロセスが実行中の場合のみ確認する（デフォルト）。
    pub confirm_close: ConfirmClose,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            blur: false,
            padding_x: 0,
            padding_y: 0,
            columns: 80,
            rows: 24,
            startup_mode: StartupMode::Windowed,
            inherit_working_directory: true,
            confirm_close: ConfirmClose::ProcessRunning,
        }
    }
}

impl WindowConfig {
    /// opacity を安全な範囲にクランプする。
    ///
    /// NaN や Inf が渡された場合はデフォルト値 1.0 を返す。
    pub fn clamped_opacity(&self) -> f32 {
        if self.opacity.is_finite() { self.opacity.clamp(0.0, 1.0) } else { 1.0 }
    }

    /// `padding_x` を安全な範囲（0〜200 ピクセル）にクランプする。
    pub fn clamped_padding_x(&self) -> u16 {
        self.padding_x.min(200)
    }

    /// `padding_y` を安全な範囲（0〜200 ピクセル）にクランプする。
    pub fn clamped_padding_y(&self) -> u16 {
        self.padding_y.min(200)
    }

    /// `columns` を安全な範囲（10〜500）にクランプする。
    pub fn clamped_columns(&self) -> u16 {
        self.columns.clamp(10, 500)
    }

    /// `rows` を安全な範囲（2〜200）にクランプする。
    pub fn clamped_rows(&self) -> u16 {
        self.rows.clamp(2, 200)
    }
}

/// ベル（BEL 0x07）の設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct BellConfig {
    /// ビジュアルベルを有効にする（画面フラッシュ）。
    pub visual: bool,
    /// macOS Dock バウンスを有効にする。
    pub dock_bounce: bool,
    /// ビジュアルベルのフェードアウト時間（ミリ秒）。
    pub duration_ms: u32,
}

impl Default for BellConfig {
    fn default() -> Self {
        Self { visual: true, dock_bounce: true, duration_ms: 150 }
    }
}

impl BellConfig {
    /// duration_ms を安全な範囲にクランプする（0 除算防止 + 長期ループ防止）。
    pub fn clamped_duration_ms(&self) -> u32 {
        self.duration_ms.clamp(1, 5000)
    }
}

/// デスクトップ通知設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct NotificationConfig {
    /// デスクトップ通知を有効にする。
    pub enabled: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// ペースト設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct PasteConfig {
    /// 複数行テキストのペースト時に確認ダイアログを表示する。
    pub confirm_multiline: bool,
}

impl Default for PasteConfig {
    fn default() -> Self {
        Self { confirm_multiline: true }
    }
}

/// スクロールバック設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ScrollbackConfig {
    /// スクロールバック履歴の最大行数。
    pub lines: u32,
}

/// シェルインテグレーション設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ShellIntegrationConfig {
    /// OSC 133 シェルインテグレーションを有効にする（プロンプトジャンプ等）。デフォルト: true。
    pub enabled: bool,
}

/// QuickSelect 設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct QuickSelectConfig {
    /// 追加の正規表現パターン（デフォルトパターンに追加して使用される）。
    pub patterns: Vec<String>,
}

/// スクロール設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ScrollingConfig {
    /// マウスホイール 1 ノッチあたりのスクロール行数倍率（デフォルト: 3、範囲: 1-100）。
    pub multiplier: u32,
    /// キー入力時にスクロールバック表示をボトムにリセットする（デフォルト: true）。
    pub scroll_to_bottom_on_keystroke: bool,
    /// PTY 出力受信時にスクロールバック表示をボトムにリセットする（デフォルト: false）。
    pub scroll_to_bottom_on_output: bool,
}

impl Default for ScrollingConfig {
    fn default() -> Self {
        Self {
            multiplier: 3,
            scroll_to_bottom_on_keystroke: true,
            scroll_to_bottom_on_output: false,
        }
    }
}

impl ScrollingConfig {
    /// multiplier を安全な範囲（1〜100）にクランプする。
    pub fn clamped_multiplier(&self) -> u32 {
        self.multiplier.clamp(1, 100)
    }
}

/// テキスト選択設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SelectionConfig {
    /// ダブルクリックで単語選択するときに、単語の一部として扱う追加文字（デフォルト: 空文字列）。
    ///
    /// 例: `"-."` を指定すると `foo-bar.baz` が1単語として選択される。
    /// 最大 256 文字。
    pub word_chars: String,
    /// テキスト選択完了時に自動的にクリップボードにコピーする（デフォルト: false）。
    pub save_to_clipboard: bool,
    /// クリップボードにコピーするとき、各行末の空白を削除する（デフォルト: true）。
    pub trim_trailing_spaces: bool,
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self { word_chars: String::new(), save_to_clipboard: false, trim_trailing_spaces: true }
    }
}

impl SelectionConfig {
    /// word_chars を最大 256 文字にクランプして返す。
    pub fn clamped_word_chars(&self) -> &str {
        let s = self.word_chars.as_str();
        // バイト境界ではなく char 境界でカット
        let end = s.char_indices().nth(256).map(|(i, _)| i).unwrap_or(s.len());
        &s[..end]
    }
}

/// マウス設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct MouseConfig {
    /// タイピング中にマウスカーソルを非表示にする（デフォルト: false）。
    pub hide_when_typing: bool,
}

/// スクロールバー設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ScrollbarConfig {
    /// スクロールバーを表示するかどうか（デフォルト: true）。
    ///
    /// `history_size == 0`（スクロールバックなし）の場合は常に非表示。
    pub enabled: bool,
    /// スクロールバーの幅（ピクセル単位、デフォルト: 8、範囲: 2-32）。
    ///
    /// 実際の描画はセル1列単位のため、視覚的なヒントとして保持する。
    pub width: u8,
}

impl Default for ScrollbarConfig {
    fn default() -> Self {
        Self { enabled: true, width: 8 }
    }
}

impl ScrollbarConfig {
    /// width を安全な範囲（2〜32）にクランプする。
    pub fn clamped_width(&self) -> u8 {
        self.width.clamp(2, 32)
    }
}

/// セキュリティ設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// フォーカス取得時に自動的に Secure Keyboard Entry を有効にする。
    ///
    /// 有効にすると、他のアプリがキーストロークをキャプチャできなくなる。
    /// パスワード入力時の保護に有効。macOS 以外のプラットフォームでは無視される。
    /// デフォルト: false
    pub auto_secure_input: bool,
}

/// カスタムリンク設定の1エントリ。
///
/// `regex` に一致したテキストをクリックすると `action` が実行される。
///
/// ```toml
/// [[links]]
/// regex = "JIRA-\\d+"
/// action = "open:https://jira.example.com/browse/$0"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinkConfig {
    /// マッチさせる正規表現パターン。
    pub regex: String,
    /// アクション文字列。`"open:<URL_TEMPLATE>"` 形式で指定する。
    /// テンプレート内の `$0` はマッチ全体、`$1` 等はキャプチャグループに展開される。
    pub action: String,
}

impl Default for MouseConfig {
    fn default() -> Self {
        Self { hide_when_typing: false }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self { auto_secure_input: false }
    }
}

impl Default for ShellIntegrationConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Default for QuickSelectConfig {
    fn default() -> Self {
        Self { patterns: Vec::new() }
    }
}

impl QuickSelectConfig {
    /// パターン一覧を最大 50 件に制限して返す。
    ///
    /// 50 件を超えるパターンを指定した場合、先頭 50 件のみが使用される。
    pub fn clamped_patterns(&self) -> &[String] {
        const MAX_PATTERNS: usize = 50;
        &self.patterns[..self.patterns.len().min(MAX_PATTERNS)]
    }
}

impl Default for ScrollbackConfig {
    fn default() -> Self {
        Self { lines: 10_000 }
    }
}

impl ScrollbackConfig {
    /// lines を安全な範囲にクランプする（0-1,000,000）。
    pub fn clamped_lines(&self) -> usize {
        (self.lines as usize).min(1_000_000)
    }
}

/// カスタムリンク設定のバリデーション定数。
const MAX_LINK_ENTRIES: usize = 32;
const MAX_LINK_REGEX_LEN: usize = 512;
const MAX_LINK_ACTION_LEN: usize = 1024;

/// SDIT 設定全体。
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// フォント設定。
    pub font: FontConfig,
    /// カラー設定。
    pub colors: ColorConfig,
    /// キーバインド設定。
    pub keybinds: KeybindConfig,
    /// macOS での Option キーの扱い。
    ///
    /// readline ショートカット（Alt+B/F/D 等）を有効にするには `"both"` を設定する。
    /// 有効な値: `"only_left"` / `"left"`, `"only_right"` / `"right"`, `"both"`, `"none"`（デフォルト）。
    /// macOS 以外のプラットフォームでは無視される。
    pub option_as_alt: OptionAsAlt,
    /// ベル設定。
    pub bell: BellConfig,
    /// ウィンドウ外観設定。
    pub window: WindowConfig,
    /// ペースト設定。
    pub paste: PasteConfig,
    /// デスクトップ通知設定。
    pub notification: NotificationConfig,
    /// カーソル設定。
    pub cursor: CursorConfig,
    /// スクロールバック設定。
    pub scrollback: ScrollbackConfig,
    /// シェルインテグレーション設定。
    pub shell_integration: ShellIntegrationConfig,
    /// QuickSelect 設定。
    pub quick_select: QuickSelectConfig,
    /// スクロール設定。
    pub scrolling: ScrollingConfig,
    /// テキスト選択設定。
    pub selection: SelectionConfig,
    /// マウス設定。
    pub mouse: MouseConfig,
    /// スクロールバー設定。
    pub scrollbar: ScrollbarConfig,
    /// セキュリティ設定。
    pub security: SecurityConfig,
    /// カスタムリンク設定。最大 32 エントリ。
    ///
    /// ```toml
    /// [[links]]
    /// regex = "JIRA-\\d+"
    /// action = "open:https://jira.example.com/browse/$0"
    /// ```
    #[serde(default)]
    pub links: Vec<LinkConfig>,
}

impl Config {
    /// カスタムリンク設定を最大 32 件・regex/action 文字列長を制限して返す。
    pub fn clamped_links(&self) -> impl Iterator<Item = &LinkConfig> {
        self.links.iter().take(MAX_LINK_ENTRIES).filter(|lc| {
            lc.regex.len() <= MAX_LINK_REGEX_LEN && lc.action.len() <= MAX_LINK_ACTION_LEN
        })
    }

    /// 設定ファイルを読み込む。
    ///
    /// ファイルが存在しない場合はデフォルト設定を返す。
    /// パースエラーの場合はログに警告を出してデフォルト設定を返す。
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(mut config) => {
                    config.keybinds.validate();
                    config.font.validate();
                    log::info!("Loaded config from {}", path.display());
                    config
                }
                Err(e) => {
                    log::warn!("Config parse error in {}: {e}", path.display());
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::info!("No config file at {}, using defaults", path.display());
                Self::default()
            }
            Err(e) => {
                log::warn!("Failed to read config {}: {e}", path.display());
                Self::default()
            }
        }
    }

    /// デフォルトの設定ファイルパスを返す。
    ///
    /// `$XDG_CONFIG_HOME/sdit/sdit.toml`（macOS では `~/.config/sdit/sdit.toml`）。
    pub fn default_path() -> PathBuf {
        dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("sdit").join("sdit.toml")
    }

    /// 設定を TOML ファイルに書き出す。
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let content = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
    }

    /// コメント付きの設定テンプレートを生成して保存する。
    ///
    /// 初回起動時や設定ファイル未存在時に呼ばれる。
    /// コメント付きの設定テンプレートを生成し、**ファイルが存在しない場合のみ**書き出す。
    ///
    /// `create_new(true)` で排他的に作成するため、TOCTOU 競合を防ぐ。
    /// ファイルが既に存在する場合は `Ok(())` を返す（何もしない）。
    pub fn save_with_comments(&self, path: &Path) -> std::io::Result<()> {
        use std::io::Write;

        let toml_body = toml::to_string_pretty(self).map_err(std::io::Error::other)?;

        let mut content = String::new();
        content.push_str("# SDIT Terminal Configuration\n");
        content.push_str("# Changes are applied automatically (hot reload).\n");
        content.push_str("#\n");
        content.push_str("# Documentation: https://github.com/user/sdit\n\n");

        // TOML ボディの各行を走査し、セクションヘッダーの前にコメントを挿入する。
        for line in toml_body.lines() {
            if line == "[font]" {
                content.push_str("# ── Font ──────────────────────────────────────────────\n");
                content.push_str("# family: font family name (default: system monospace)\n");
                content.push_str("# size: font size in pixels (1.0 - 200.0, default: 14.0)\n");
                content
                    .push_str("# line_height: line height multiplier (0.5 - 5.0, default: 1.2)\n");
                content.push_str(
                    "# fallback_families: list of fallback font families (e.g. for CJK)\n",
                );
                content.push_str(
                    "# codepoint_map: per-codepoint-range font override (max 64 entries)\n",
                );
                content.push_str(
                    "#   e.g. [font.codepoint_map] \"U+3000-U+9FFF\" = \"Noto Sans CJK\"\n",
                );
                content.push_str(
                    "# variation: OpenType font variation (max 16 entries, future support)\n",
                );
                content.push_str("#   e.g. [font.variation] wght = 700.0\n");
                content.push_str(
                    "# feature: OpenType font feature on/off (max 32 entries, future support)\n",
                );
                content.push_str("#   e.g. [font.feature] calt = true\n");
                content.push_str(
                    "# adjust: fine-tune cell metrics (cell_width, cell_height, baseline in pixels)\n",
                );
                content.push_str(
                    "#   e.g. [font.adjust] cell_width = 1.0  cell_height = 0.0  baseline = -1.0\n",
                );
            } else if line == "[colors]" {
                content.push('\n');
                content.push_str("# ── Colors ─────────────────────────────────────────────\n");
                content.push_str("# theme: built-in color theme name\n");
                content.push_str(
                    "#   available: \"catppuccin-mocha\", \"catppuccin-latte\", \"gruvbox-dark\"\n",
                );
                content.push_str("# selection_foreground: text selection foreground color as hex \"#RRGGBB\"; omit to use inverted color\n");
                content.push_str("# selection_background: text selection background color as hex \"#RRGGBB\"; omit to use inverted color\n");
                content.push_str("# minimum_contrast: minimum WCAG 2.0 contrast ratio for cell rendering (1.0 = disabled, max 21.0)\n");
                content.push_str("#   fg color is auto-adjusted when the contrast ratio falls below this value\n");
                content.push_str("#   e.g. minimum_contrast = 4.5  # WCAG AA level\n");
            } else if line == "[keybinds]" || line == "[[keybinds]]" {
                content.push('\n');
                content.push_str("# ── Keybinds ────────────────────────────────────────────\n");
                content.push_str(
                    "# Each entry: key, mods (\"super\", \"ctrl\", \"shift\", \"alt\", combined with \"|\"), action\n",
                );
                content
                    .push_str("# Example: key = \"n\", mods = \"super\", action = \"NewWindow\"\n");
            } else if line.starts_with("option_as_alt") {
                content.push('\n');
                content.push_str("# ── macOS Option Key ────────────────────────────────────\n");
                content.push_str("# option_as_alt: treat Option key as Alt for readline shortcuts (Alt+B/F/D etc.)\n");
                content.push_str(
                    "#   values: \"none\" (default), \"both\", \"only_left\" / \"left\", \"only_right\" / \"right\"\n",
                );
            } else if line == "[bell]" {
                content.push('\n');
                content.push_str("# ── Bell ──────────────────────────────────────────────\n");
                content.push_str(
                    "# visual: flash the screen when BEL (0x07) is received (default: true)\n",
                );
                content.push_str("# dock_bounce: bounce the Dock icon when BEL is received while unfocused (default: true, macOS only)\n");
                content.push_str(
                    "# duration_ms: visual bell fade-out duration in milliseconds (default: 150)\n",
                );
            } else if line == "[window]" {
                content.push('\n');
                content.push_str("# ── Window ─────────────────────────────────────────────\n");
                content.push_str(
                    "# opacity: background opacity (0.0 = fully transparent, 1.0 = opaque, default: 1.0)\n",
                );
                content.push_str(
                    "# blur: enable background blur effect (macOS only, default: false)\n",
                );
                content.push_str(
                    "# padding_x: horizontal padding between grid and window edge in pixels (0-200, default: 0)\n",
                );
                content.push_str(
                    "# padding_y: vertical padding between grid and window edge in pixels (0-200, default: 0)\n",
                );
                content.push_str(
                    "# columns: initial terminal width in columns (10-500, default: 80)\n",
                );
                content.push_str("# rows: initial terminal height in rows (2-200, default: 24)\n");
                content.push_str("# startup_mode: initial window state: \"Windowed\" (default), \"Maximized\", \"Fullscreen\"\n");
                content.push_str("# inherit_working_directory: inherit active session's working directory for new sessions/windows (default: true)\n");
                content.push_str("# confirm_close: show confirmation dialog when closing a session (default: \"process_running\")\n");
                content.push_str("#   \"never\": close without confirmation\n");
                content.push_str("#   \"always\": always show confirmation\n");
                content.push_str("#   \"process_running\": show confirmation only when a foreground process is running\n");
            } else if line == "[paste]" {
                content.push('\n');
                content.push_str("# ── Paste ─────────────────────────────────────────────\n");
                content.push_str("# confirm_multiline: show confirmation dialog when pasting text containing newlines (default: true)\n");
            } else if line == "[notification]" {
                content.push('\n');
                content.push_str("# ── Notification ──────────────────────────────────────\n");
                content.push_str("# enabled: show desktop notifications from OSC 9/99 sequences (default: true)\n");
            } else if line == "[cursor]" {
                content.push('\n');
                content.push_str("# ── Cursor ─────────────────────────────────────────────\n");
                content.push_str(
                    "# style: cursor shape: \"block\" (default), \"underline\", \"bar\"\n",
                );
                content.push_str("# blinking: enable cursor blinking (default: false)\n");
                content.push_str("# color: cursor color as hex string (e.g. \"#ff6600\"); omit to use theme foreground\n");
            } else if line == "[scrollback]" {
                content.push('\n');
                content.push_str("# ── Scrollback ──────────────────────────────────────────\n");
                content.push_str(
                    "# lines: maximum number of scrollback lines (default: 10000, range: 0-1000000)\n",
                );
            } else if line == "[shell_integration]" {
                content.push('\n');
                content.push_str("# ── Shell Integration ──────────────────────────────────────\n");
                content.push_str("# enabled: enable OSC 133 shell integration for prompt navigation (default: true)\n");
            } else if line == "[quick_select]" {
                content.push('\n');
                content.push_str("# ── Quick Select ───────────────────────────────────────────\n");
                content.push_str(
                    "# patterns: additional regex patterns to match in Quick Select mode\n",
                );
                content.push_str(
                    "# Example: patterns = [\"[A-Z]+-\\\\d+\"]  # matches JIRA issue IDs\n",
                );
            } else if line == "[scrolling]" {
                content.push('\n');
                content
                    .push_str("# ── Scrolling ───────────────────────────────────────────────\n");
                content.push_str(
                    "# multiplier: scroll lines per mouse wheel notch (1-100, default: 3)\n",
                );
                content.push_str("# scroll_to_bottom_on_keystroke: scroll to bottom when a key is pressed (default: true)\n");
                content.push_str("# scroll_to_bottom_on_output: scroll to bottom when new output is received (default: false)\n");
            } else if line == "[selection]" {
                content.push('\n');
                content
                    .push_str("# ── Selection ───────────────────────────────────────────────\n");
                content.push_str("# word_chars: extra characters treated as part of a word in double-click selection (default: \"\")\n");
                content.push_str("# save_to_clipboard: auto-copy selected text to clipboard on mouse release (default: false)\n");
                content.push_str("# trim_trailing_spaces: remove trailing whitespace from each line when copying to clipboard (default: true)\n");
            } else if line == "[mouse]" {
                content.push('\n');
                content.push_str("# ── Mouse ──────────────────────────────────────────────────\n");
                content.push_str(
                    "# hide_when_typing: hide mouse cursor while typing (default: false)\n",
                );
            } else if line == "[scrollbar]" {
                content.push('\n');
                content.push_str("# ── Scrollbar ──────────────────────────────────────────────\n");
                content.push_str(
                    "# enabled: show scrollbar when scrollback history exists (default: true)\n",
                );
                content.push_str("# width: scrollbar width hint in pixels (2-32, default: 8)\n");
            } else if line == "[security]" {
                content.push('\n');
                content.push_str("# ── Security ───────────────────────────────────────────────\n");
                content.push_str("# auto_secure_input: automatically enable Secure Keyboard Entry on focus (macOS only, default: false)\n");
                content.push_str(
                    "#   prevents other apps from capturing keystrokes while SDIT is focused\n",
                );
            } else if line == "[[links]]" {
                content.push('\n');
                content.push_str("# ── Custom Links ───────────────────────────────────────────\n");
                content.push_str(
                    "# Custom link patterns. Each entry defines a regex and an action.\n",
                );
                content.push_str(
                    "# regex: regular expression pattern (max 512 chars, max 32 entries)\n",
                );
                content.push_str("# action: \"open:<URL_TEMPLATE>\" where $0 = full match, $1/$2 = capture groups\n");
                content.push_str("# Example:\n");
                content.push_str("#   [[links]]\n");
                content.push_str("#   regex = \"JIRA-\\\\d+\"\n");
                content.push_str("#   action = \"open:https://jira.example.com/browse/$0\"\n");
            }
            content.push_str(line);
            content.push('\n');
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match std::fs::OpenOptions::new().write(true).create_new(true).open(path) {
            Ok(mut file) => file.write_all(content.as_bytes()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollbar_config_default() {
        let cfg = ScrollbarConfig::default();
        assert!(cfg.enabled, "enabled のデフォルトは true");
        assert_eq!(cfg.width, 8, "width のデフォルトは 8");
    }

    #[test]
    fn scrollbar_config_clamped_width() {
        let cfg = ScrollbarConfig { enabled: true, width: 1 };
        assert_eq!(cfg.clamped_width(), 2, "width=1 はクランプされて 2 になる");

        let cfg = ScrollbarConfig { enabled: true, width: 8 };
        assert_eq!(cfg.clamped_width(), 8, "width=8 はそのまま");

        let cfg = ScrollbarConfig { enabled: true, width: 33 };
        assert_eq!(cfg.clamped_width(), 32, "width=33 はクランプされて 32 になる");
    }

    #[test]
    fn scrollbar_config_deserialize() {
        let toml_str = "[scrollbar]\nenabled = false\nwidth = 12\n";
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert!(!cfg.scrollbar.enabled);
        assert_eq!(cfg.scrollbar.width, 12);
    }

    #[test]
    fn scrollbar_config_default_in_full_config() {
        let cfg = Config::default();
        assert!(cfg.scrollbar.enabled);
        assert_eq!(cfg.scrollbar.width, 8);
    }

    #[test]
    fn security_config_default() {
        let cfg = SecurityConfig::default();
        assert!(!cfg.auto_secure_input, "auto_secure_input のデフォルトは false");
    }

    #[test]
    fn security_config_deserialize() {
        let toml_str = "[security]\nauto_secure_input = true\n";
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert!(cfg.security.auto_secure_input);
    }

    #[test]
    fn security_config_default_in_full_config() {
        let cfg = Config::default();
        assert!(!cfg.security.auto_secure_input);
    }

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();
        assert!(!config.font.family.is_empty());
        assert!(config.font.size > 0.0);
    }

    #[test]
    fn option_as_alt_default_is_none() {
        let config = Config::default();
        assert_eq!(config.option_as_alt, OptionAsAlt::None);
    }

    #[test]
    fn option_as_alt_deserialize_canonical() {
        let cases: &[(&str, OptionAsAlt)] = &[
            (r#"option_as_alt = "none""#, OptionAsAlt::None),
            (r#"option_as_alt = "both""#, OptionAsAlt::Both),
            (r#"option_as_alt = "only_left""#, OptionAsAlt::OnlyLeft),
            (r#"option_as_alt = "only_right""#, OptionAsAlt::OnlyRight),
        ];
        for (toml_str, expected) in cases {
            let config: Config = toml::from_str(toml_str).unwrap();
            assert_eq!(config.option_as_alt, *expected, "failed for: {toml_str}");
        }
    }

    #[test]
    fn option_as_alt_deserialize_alias() {
        let left: Config = toml::from_str(r#"option_as_alt = "left""#).unwrap();
        assert_eq!(left.option_as_alt, OptionAsAlt::OnlyLeft);

        let right: Config = toml::from_str(r#"option_as_alt = "right""#).unwrap();
        assert_eq!(right.option_as_alt, OptionAsAlt::OnlyRight);
    }

    #[test]
    fn option_as_alt_serialize() {
        let config = Config { option_as_alt: OptionAsAlt::Both, ..Default::default() };
        let serialized = toml::to_string(&config).unwrap();
        assert!(
            serialized.contains("option_as_alt = \"both\""),
            "expected serialized form: {serialized}"
        );
    }

    #[test]
    fn load_nonexistent_returns_default() {
        let config = Config::load(Path::new("/nonexistent/path/sdit.toml"));
        assert!(!config.font.family.is_empty());
        assert!((config.font.size - 14.0).abs() < f32::EPSILON);
    }

    #[test]
    fn deserialize_full_config() {
        let toml_str = r#"
[font]
family = "JetBrains Mono"
size = 16.0
line_height = 1.3
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.font.family, "JetBrains Mono");
        assert!((config.font.size - 16.0).abs() < f32::EPSILON);
        assert!((config.font.line_height - 1.3).abs() < f32::EPSILON);
    }

    #[test]
    fn deserialize_empty_uses_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert!(!config.font.family.is_empty());
        assert!((config.font.size - 14.0).abs() < f32::EPSILON);
    }

    #[test]
    fn default_path_not_empty() {
        let path = Config::default_path();
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn config_save_and_load_roundtrip() {
        let config = Config::default();
        let path = std::path::PathBuf::from("tmp/test-config-roundtrip.toml");
        std::fs::create_dir_all("tmp").expect("tmp dir");
        config.save(&path).expect("save failed");
        let loaded = Config::load(&path);
        assert!(
            (loaded.font.size - config.font.size).abs() < f32::EPSILON,
            "font.size mismatch: {} vs {}",
            loaded.font.size,
            config.font.size
        );
        assert_eq!(loaded.font.family, config.font.family);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn bell_config_default() {
        let bell = BellConfig::default();
        assert!(bell.visual);
        assert!(bell.dock_bounce);
        assert_eq!(bell.duration_ms, 150);
    }

    #[test]
    fn bell_config_deserialize() {
        let toml_str = "[bell]\nvisual = false\ndock_bounce = false\nduration_ms = 200\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(!config.bell.visual);
        assert!(!config.bell.dock_bounce);
        assert_eq!(config.bell.duration_ms, 200);
    }

    #[test]
    fn bell_config_partial_deserialize() {
        // 部分指定のとき残りはデフォルト補完
        let toml_str = "[bell]\nduration_ms = 300\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.bell.visual); // default: true
        assert!(config.bell.dock_bounce); // default: true
        assert_eq!(config.bell.duration_ms, 300);
    }

    #[test]
    fn bell_duration_clamp_zero() {
        let bell = BellConfig { duration_ms: 0, ..Default::default() };
        assert_eq!(bell.clamped_duration_ms(), 1);
    }

    #[test]
    fn bell_duration_clamp_max() {
        let bell = BellConfig { duration_ms: 999_999, ..Default::default() };
        assert_eq!(bell.clamped_duration_ms(), 5000);
    }

    #[test]
    fn window_config_default() {
        let wc = WindowConfig::default();
        assert!((wc.opacity - 1.0).abs() < f32::EPSILON);
        assert!(!wc.blur);
        assert_eq!(wc.padding_x, 0);
        assert_eq!(wc.padding_y, 0);
    }

    #[test]
    fn window_config_deserialize() {
        let toml_str = "[window]\nopacity = 0.8\nblur = true\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!((config.window.opacity - 0.8).abs() < f32::EPSILON);
        assert!(config.window.blur);
    }

    #[test]
    fn window_config_padding_deserialize() {
        let toml_str = "[window]\npadding_x = 8\npadding_y = 4\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.window.padding_x, 8);
        assert_eq!(config.window.padding_y, 4);
    }

    #[test]
    fn window_config_partial_deserialize() {
        let toml_str = "[window]\nopacity = 0.5\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!((config.window.opacity - 0.5).abs() < f32::EPSILON);
        assert!(!config.window.blur); // default
    }

    #[test]
    fn window_opacity_clamp() {
        let wc = WindowConfig { opacity: -0.5, ..Default::default() };
        assert!((wc.clamped_opacity() - 0.0).abs() < f32::EPSILON);
        let wc = WindowConfig { opacity: 2.0, ..Default::default() };
        assert!((wc.clamped_opacity() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn window_padding_clamp() {
        // 200 を超える値は 200 にクランプされる
        let wc = WindowConfig { padding_x: 500, padding_y: 300, ..Default::default() };
        assert_eq!(wc.clamped_padding_x(), 200);
        assert_eq!(wc.clamped_padding_y(), 200);
        // 範囲内の値はそのまま
        let wc2 = WindowConfig { padding_x: 8, padding_y: 4, ..Default::default() };
        assert_eq!(wc2.clamped_padding_x(), 8);
        assert_eq!(wc2.clamped_padding_y(), 4);
    }

    #[test]
    fn window_opacity_clamp_nan_inf() {
        let wc = WindowConfig { opacity: f32::NAN, ..Default::default() };
        assert!((wc.clamped_opacity() - 1.0).abs() < f32::EPSILON);
        let wc = WindowConfig { opacity: f32::INFINITY, ..Default::default() };
        assert!((wc.clamped_opacity() - 1.0).abs() < f32::EPSILON);
        let wc = WindowConfig { opacity: f32::NEG_INFINITY, ..Default::default() };
        assert!((wc.clamped_opacity() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn paste_config_default() {
        let pc = PasteConfig::default();
        assert!(pc.confirm_multiline);
    }

    #[test]
    fn paste_config_deserialize() {
        let toml_str = "[paste]\nconfirm_multiline = false\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(!config.paste.confirm_multiline);
    }

    #[test]
    fn notification_config_default() {
        let nc = NotificationConfig::default();
        assert!(nc.enabled);
    }

    #[test]
    fn notification_config_deserialize() {
        let toml_str = "[notification]\nenabled = false\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(!config.notification.enabled);
    }

    #[test]
    fn config_save_with_comments_is_parseable() {
        let config = Config::default();
        let path = std::path::PathBuf::from("tmp/test-config-comments.toml");
        std::fs::create_dir_all("tmp").expect("tmp dir");
        // create_new(true) を使うため、既存ファイルを先に削除する
        let _ = std::fs::remove_file(&path);
        config.save_with_comments(&path).expect("save failed");
        let loaded = Config::load(&path);
        assert!(
            (loaded.font.size - config.font.size).abs() < f32::EPSILON,
            "font.size mismatch after comment-save: {} vs {}",
            loaded.font.size,
            config.font.size
        );
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# SDIT"), "expected '# SDIT' comment header");
        assert!(content.contains("[font]"), "expected [font] section");
        let _ = std::fs::remove_file(&path);
    }

    // -----------------------------------------------------------------------
    // CursorConfig テスト
    // -----------------------------------------------------------------------

    #[test]
    fn cursor_config_default() {
        let cc = CursorConfig::default();
        assert_eq!(cc.style, CursorStyleConfig::Block);
        assert!(!cc.blinking);
        assert!(cc.color.is_none());
    }

    #[test]
    fn cursor_config_deserialize_full() {
        let toml_str = "[cursor]\nstyle = \"bar\"\nblinking = true\ncolor = \"#ff6600\"\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.cursor.style, CursorStyleConfig::Bar);
        assert!(config.cursor.blinking);
        assert_eq!(config.cursor.color.as_deref(), Some("#ff6600"));
    }

    #[test]
    fn cursor_config_deserialize_partial() {
        let toml_str = "[cursor]\nstyle = \"underline\"\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.cursor.style, CursorStyleConfig::Underline);
        assert!(!config.cursor.blinking); // default
        assert!(config.cursor.color.is_none()); // default
    }

    #[test]
    fn cursor_config_deserialize_empty_uses_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.cursor.style, CursorStyleConfig::Block);
        assert!(!config.cursor.blinking);
        assert!(config.cursor.color.is_none());
    }

    #[test]
    fn cursor_style_config_converts_to_cursor_style() {
        use crate::terminal::CursorStyle;
        assert_eq!(CursorStyle::from(CursorStyleConfig::Block), CursorStyle::Block);
        assert_eq!(CursorStyle::from(CursorStyleConfig::Underline), CursorStyle::Underline);
        assert_eq!(CursorStyle::from(CursorStyleConfig::Bar), CursorStyle::Bar);
    }

    // -----------------------------------------------------------------------
    // ScrollbackConfig テスト
    // -----------------------------------------------------------------------

    #[test]
    fn scrollback_config_default() {
        let sc = ScrollbackConfig::default();
        assert_eq!(sc.lines, 10_000);
    }

    #[test]
    fn scrollback_config_deserialize() {
        let toml_str = "[scrollback]\nlines = 50000\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.scrollback.lines, 50_000);
    }

    #[test]
    fn scrollback_clamped_lines_zero() {
        let sc = ScrollbackConfig { lines: 0 };
        assert_eq!(sc.clamped_lines(), 0);
    }

    #[test]
    fn scrollback_clamped_lines_over_max() {
        let sc = ScrollbackConfig { lines: 2_000_000 };
        assert_eq!(sc.clamped_lines(), 1_000_000);
    }

    #[test]
    fn scrollback_config_empty_uses_default() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.scrollback.lines, 10_000);
    }

    // -----------------------------------------------------------------------
    // WindowConfig columns/rows テスト
    // -----------------------------------------------------------------------

    #[test]
    fn window_config_columns_rows_default() {
        let wc = WindowConfig::default();
        assert_eq!(wc.columns, 80);
        assert_eq!(wc.rows, 24);
    }

    #[test]
    fn window_config_columns_rows_deserialize() {
        let toml_str = "[window]\ncolumns = 120\nrows = 36\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.window.columns, 120);
        assert_eq!(config.window.rows, 36);
    }

    #[test]
    fn window_config_columns_clamp() {
        let wc = WindowConfig { columns: 5, ..Default::default() };
        assert_eq!(wc.clamped_columns(), 10);
        let wc = WindowConfig { columns: 600, ..Default::default() };
        assert_eq!(wc.clamped_columns(), 500);
        let wc = WindowConfig { columns: 80, ..Default::default() };
        assert_eq!(wc.clamped_columns(), 80);
    }

    #[test]
    fn window_config_rows_clamp() {
        let wc = WindowConfig { rows: 1, ..Default::default() };
        assert_eq!(wc.clamped_rows(), 2);
        let wc = WindowConfig { rows: 300, ..Default::default() };
        assert_eq!(wc.clamped_rows(), 200);
        let wc = WindowConfig { rows: 24, ..Default::default() };
        assert_eq!(wc.clamped_rows(), 24);
    }

    // -----------------------------------------------------------------------
    // ScrollingConfig のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn scrolling_config_default() {
        let sc = ScrollingConfig::default();
        assert_eq!(sc.multiplier, 3);
        assert_eq!(sc.clamped_multiplier(), 3);
    }

    #[test]
    fn scrolling_config_clamp_min() {
        let sc = ScrollingConfig { multiplier: 0, ..Default::default() };
        assert_eq!(sc.clamped_multiplier(), 1);
    }

    #[test]
    fn scrolling_config_clamp_max() {
        let sc = ScrollingConfig { multiplier: 999, ..Default::default() };
        assert_eq!(sc.clamped_multiplier(), 100);
    }

    #[test]
    fn scrolling_config_deserialize() {
        let toml_str = "[scrolling]\nmultiplier = 5\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.scrolling.multiplier, 5);
    }

    // -----------------------------------------------------------------------
    // SelectionConfig のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn selection_config_default() {
        let sc = SelectionConfig::default();
        assert!(sc.word_chars.is_empty());
        assert!(!sc.save_to_clipboard);
    }

    #[test]
    fn selection_config_word_chars_clamp() {
        // 256 文字以内はそのまま
        let s256: String = "a".repeat(256);
        let sc = SelectionConfig { word_chars: s256.clone(), ..Default::default() };
        assert_eq!(sc.clamped_word_chars().chars().count(), 256);

        // 257 文字は 256 文字にクランプ
        let s257: String = "b".repeat(257);
        let sc2 = SelectionConfig { word_chars: s257, ..Default::default() };
        assert_eq!(sc2.clamped_word_chars().chars().count(), 256);
    }

    #[test]
    fn selection_config_deserialize() {
        let toml_str = "[selection]\nword_chars = \"-.\"\nsave_to_clipboard = true\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.selection.word_chars, "-.");
        assert!(config.selection.save_to_clipboard);
    }

    // -----------------------------------------------------------------------
    // MouseConfig のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn mouse_config_default() {
        let mc = MouseConfig::default();
        assert!(!mc.hide_when_typing);
    }

    #[test]
    fn mouse_config_deserialize() {
        let toml_str = "[mouse]\nhide_when_typing = true\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.mouse.hide_when_typing);
    }

    // -----------------------------------------------------------------------
    // LinkConfig のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn link_config_deserialize() {
        let toml_str = r#"
[[links]]
regex = "JIRA-\\d+"
action = "open:https://jira.example.com/browse/$0"

[[links]]
regex = "GH-\\d+"
action = "open:https://github.com/issues/$0"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.links.len(), 2);
        assert_eq!(config.links[0].regex, "JIRA-\\d+");
        assert_eq!(config.links[0].action, "open:https://jira.example.com/browse/$0");
        assert_eq!(config.links[1].regex, "GH-\\d+");
    }

    #[test]
    fn link_config_empty_by_default() {
        let config: Config = toml::from_str("").unwrap();
        assert!(config.links.is_empty());
    }

    #[test]
    fn link_config_entry_limit() {
        // 33 エントリを設定しても clamped_links は 32 件しか返さない
        let entries: String = (0..33)
            .map(|i| {
                format!("[[links]]\nregex = \"PAT-{i}\"\naction = \"open:https://example.com\"\n")
            })
            .collect();
        let config: Config = toml::from_str(&entries).unwrap();
        assert_eq!(config.links.len(), 33);
        assert_eq!(config.clamped_links().count(), 32);
    }

    #[test]
    fn link_config_regex_length_limit() {
        // regex が 512 文字を超えるエントリは clamped_links に含まれない
        let long_regex = "a".repeat(513);
        let toml_str =
            format!("[[links]]\nregex = \"{long_regex}\"\naction = \"open:https://example.com\"\n");
        let config: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.clamped_links().count(), 0);
    }

    #[test]
    fn link_config_action_length_limit() {
        // action が 1024 文字を超えるエントリは clamped_links に含まれない
        let long_action = format!("open:https://example.com/{}", "a".repeat(1000));
        let toml_str = format!("[[links]]\nregex = \"PAT\"\naction = \"{long_action}\"\n");
        let config: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.clamped_links().count(), 0);
    }

    // -----------------------------------------------------------------------
    // Phase 16 新設定のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn startup_mode_default_is_windowed() {
        let wc = WindowConfig::default();
        assert_eq!(wc.startup_mode, StartupMode::Windowed);
    }

    #[test]
    fn startup_mode_deserialize() {
        let cases: &[(&str, StartupMode)] = &[
            ("[window]\nstartup_mode = \"Windowed\"\n", StartupMode::Windowed),
            ("[window]\nstartup_mode = \"Maximized\"\n", StartupMode::Maximized),
            ("[window]\nstartup_mode = \"Fullscreen\"\n", StartupMode::Fullscreen),
        ];
        for (toml_str, expected) in cases {
            let config: Config = toml::from_str(toml_str).unwrap();
            assert_eq!(config.window.startup_mode, *expected, "failed for: {toml_str}");
        }
    }

    #[test]
    fn window_config_inherit_working_directory_default() {
        let wc = WindowConfig::default();
        assert!(wc.inherit_working_directory);
    }

    #[test]
    fn window_config_inherit_working_directory_deserialize() {
        let toml_str = "[window]\ninherit_working_directory = false\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(!config.window.inherit_working_directory);
    }

    #[test]
    fn selection_trim_trailing_spaces_default() {
        let sc = SelectionConfig::default();
        assert!(sc.trim_trailing_spaces);
    }

    #[test]
    fn selection_trim_trailing_spaces_deserialize() {
        let toml_str = "[selection]\ntrim_trailing_spaces = false\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(!config.selection.trim_trailing_spaces);
    }

    #[test]
    fn scrolling_scroll_to_bottom_default() {
        let sc = ScrollingConfig::default();
        assert!(sc.scroll_to_bottom_on_keystroke);
        assert!(!sc.scroll_to_bottom_on_output);
    }

    #[test]
    fn scrolling_scroll_to_bottom_deserialize() {
        let toml_str = "[scrolling]\nscroll_to_bottom_on_keystroke = false\nscroll_to_bottom_on_output = true\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(!config.scrolling.scroll_to_bottom_on_keystroke);
        assert!(config.scrolling.scroll_to_bottom_on_output);
    }

    // -----------------------------------------------------------------------
    // ConfirmClose のテスト
    // -----------------------------------------------------------------------

    #[test]
    fn confirm_close_default_is_process_running() {
        let wc = WindowConfig::default();
        assert_eq!(wc.confirm_close, ConfirmClose::ProcessRunning);
    }

    #[test]
    fn confirm_close_deserialize_never() {
        let toml_str = "[window]\nconfirm_close = \"never\"\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.window.confirm_close, ConfirmClose::Never);
    }

    #[test]
    fn confirm_close_deserialize_always() {
        let toml_str = "[window]\nconfirm_close = \"always\"\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.window.confirm_close, ConfirmClose::Always);
    }

    #[test]
    fn confirm_close_deserialize_process_running() {
        let toml_str = "[window]\nconfirm_close = \"process_running\"\n";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.window.confirm_close, ConfirmClose::ProcessRunning);
    }

    #[test]
    fn confirm_close_default_in_full_config() {
        let cfg = Config::default();
        assert_eq!(cfg.window.confirm_close, ConfirmClose::ProcessRunning);
    }
}
