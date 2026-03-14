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

/// ウィンドウデコレーション（タイトルバー・枠）の設定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decorations {
    /// ネイティブのウィンドウデコレーションを表示する（デフォルト）。
    Full,
    /// ウィンドウデコレーションなし（ボーダーレス）。
    None,
}

impl Default for Decorations {
    fn default() -> Self {
        Self::Full
    }
}

/// 背景画像のフィット方法。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundImageFit {
    /// アスペクト比を保持して画像全体が見えるようにする。
    Contain,
    /// アスペクト比を保持してウィンドウ全体を覆う。
    #[default]
    Cover,
    /// アスペクト比を無視してウィンドウ全体に引き伸ばす。
    Fill,
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
    /// ウィンドウデコレーション設定: "full" (デフォルト) = ネイティブ装飾あり、"none" = ボーダーレス。
    pub decorations: Decorations,
    /// ウィンドウを常に最前面に表示する（デフォルト: false）。
    pub always_on_top: bool,
    /// 次回起動時にセッション（ウィンドウ・タブ構成）を復帰する（デフォルト: true）。
    pub restore_session: bool,
    /// 背景画像のパス（省略時: なし）。PNG/JPEG/WebP をサポート。
    ///
    /// `~` で始まる場合はホームディレクトリに展開する。
    /// ファイルが見つからない場合は warn ログを出してスキップする。
    pub background_image: Option<String>,
    /// 背景画像の不透明度（0.0-1.0、デフォルト: 0.3）。
    pub background_image_opacity: f32,
    /// 背景画像のフィット方法（デフォルト: cover）。
    pub background_image_fit: BackgroundImageFit,
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
            decorations: Decorations::Full,
            always_on_top: false,
            restore_session: true,
            background_image: None,
            background_image_opacity: 0.3,
            background_image_fit: BackgroundImageFit::Cover,
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

    /// `background_image_opacity` を安全な範囲にクランプする。
    ///
    /// NaN や Inf が渡された場合はデフォルト値 0.3 を返す。
    pub fn clamped_background_image_opacity(&self) -> f32 {
        if self.background_image_opacity.is_finite() {
            self.background_image_opacity.clamp(0.0, 1.0)
        } else {
            0.3
        }
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
    /// クリップボードにコピーする際の文字変換マップ（デフォルト: 空）。
    ///
    /// キー: Unicode コードポイント範囲（例: `"U+2500-U+257F"`）、値: 置換文字列。
    /// 各マッチした文字は値の文字列に置換される。最大 64 エントリ。
    ///
    /// 例: ボックス描画文字を ASCII に変換
    /// ```toml
    /// [selection.clipboard_codepoint_map]
    /// "U+2500-U+257F" = " "
    /// ```
    #[serde(default)]
    pub clipboard_codepoint_map: std::collections::HashMap<String, String>,
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self {
            word_chars: String::new(),
            save_to_clipboard: false,
            trim_trailing_spaces: true,
            clipboard_codepoint_map: std::collections::HashMap::new(),
        }
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

    /// `clipboard_codepoint_map` を使って文字変換を適用したテキストを返す。
    ///
    /// マップのエントリ数は最大 64 に制限する。
    /// キーは "U+XXXX-U+YYYY" または "U+XXXX" 形式。
    /// replacement 文字列の最大長は 256 文字（超えるエントリは除外して warn ログを出す）。
    /// 出力サイズが入力の 10 倍を超えたら処理を中断する（DoS 防止）。
    pub fn apply_codepoint_map(&self, text: &str) -> String {
        if self.clipboard_codepoint_map.is_empty() {
            return text.to_owned();
        }

        /// replacement 文字列の最大長（chars 単位）。
        const MAX_REPLACEMENT_CHARS: usize = 256;
        /// 出力サイズ上限: 入力の 10 倍まで。
        const MAX_OUTPUT_MULTIPLIER: usize = 10;

        // キーをパースしてレンジリストを構築（最大 64 エントリ）
        // replacement が長すぎるエントリは除外して warn する。
        let ranges: Vec<(u32, u32, &str)> = self
            .clipboard_codepoint_map
            .iter()
            .take(64)
            .filter_map(|(range_str, replacement)| {
                if replacement.chars().count() > MAX_REPLACEMENT_CHARS {
                    log::warn!(
                        "apply_codepoint_map: replacement for \"{}\" exceeds {} chars, skipping",
                        range_str,
                        MAX_REPLACEMENT_CHARS
                    );
                    return None;
                }
                parse_clipboard_range(range_str)
                    .map(|(start, end)| (start, end, replacement.as_str()))
            })
            .collect();

        if ranges.is_empty() {
            return text.to_owned();
        }

        let max_output_len = text.len().saturating_mul(MAX_OUTPUT_MULTIPLIER);
        let mut result = String::with_capacity(text.len());
        for c in text.chars() {
            // 出力膨張チェック
            if result.len() > max_output_len {
                log::warn!(
                    "apply_codepoint_map: output exceeded {}x input size, truncating",
                    MAX_OUTPUT_MULTIPLIER
                );
                break;
            }
            let cp = c as u32;
            let mut replaced = false;
            for &(start, end, replacement) in &ranges {
                if cp >= start && cp <= end {
                    result.push_str(replacement);
                    replaced = true;
                    break;
                }
            }
            if !replaced {
                result.push(c);
            }
        }
        result
    }
}

/// "U+XXXX-U+YYYY" または "U+XXXX" 形式のレンジ文字列をパースして (start, end) を返す。
fn parse_clipboard_range(range_str: &str) -> Option<(u32, u32)> {
    let range_str = range_str.trim();
    // "U+XXXX-U+YYYY" 形式を試みる（"-U+" をセパレータとして分割）
    if let Some(idx) = range_str.find("-U+").or_else(|| range_str.find("-u+")) {
        let start_str = &range_str[..idx];
        let end_str = &range_str[idx + 1..];
        let start = parse_clipboard_codepoint(start_str)?;
        let end = parse_clipboard_codepoint(end_str)?;
        if start <= end {
            return Some((start, end));
        }
        return None;
    }
    // 単一コードポイント: "U+XXXX"
    let cp = parse_clipboard_codepoint(range_str)?;
    Some((cp, cp))
}

/// "U+XXXX" または "XXXX" 形式のコードポイント文字列を u32 にパースする。
///
/// - 空文字列の場合は `None` を返す
/// - hex 部分が 8 文字を超える場合は `None` を返す
/// - 全文字が ASCII 16 進数でない場合は `None` を返す
pub(super) fn parse_clipboard_codepoint(s: &str) -> Option<u32> {
    let s = s.trim();
    let hex = if let Some(stripped) = s.strip_prefix("U+").or_else(|| s.strip_prefix("u+")) {
        stripped
    } else {
        s
    };
    // 空文字列チェック
    if hex.is_empty() {
        return None;
    }
    // 8 文字超チェック（U+10FFFF は 6 桁だが、余裕を持って 8 文字まで許可）
    if hex.len() > 8 {
        return None;
    }
    // 全文字が ASCII 16 進数であることを確認
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(hex, 16).ok().filter(|&cp| cp <= 0x10FFFF)
}

/// 右クリック時の動作設定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RightClickAction {
    /// コンテキストメニューを表示する（デフォルト）。
    ContextMenu,
    /// クリップボードからペーストする。
    Paste,
    /// 何もしない。
    None,
}

impl Default for RightClickAction {
    fn default() -> Self {
        Self::ContextMenu
    }
}

/// マウス設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct MouseConfig {
    /// タイピング中にマウスカーソルを非表示にする（デフォルト: false）。
    pub hide_when_typing: bool,
    /// 右クリック時の動作: "context_menu" (デフォルト), "paste", "none"。
    pub right_click_action: RightClickAction,
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
        Self { hide_when_typing: false, right_click_action: RightClickAction::ContextMenu }
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
                    "#   available: \"catppuccin-mocha\", \"catppuccin-latte\", \"gruvbox-dark\",\n",
                );
                content.push_str(
                    "#              \"solarized-dark\", \"solarized-light\", \"dracula\", \"nord\", \"one-dark\", \"tokyo-night\"\n",
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
                content.push_str("# decorations: window decoration mode (default: \"full\")\n");
                content.push_str("#   \"full\": native window decorations (title bar, borders)\n");
                content.push_str("#   \"none\": borderless window (no title bar)\n");
                content.push_str(
                    "# always_on_top: keep window above all other windows (default: false)\n",
                );
                content.push_str(
                    "# restore_session: restore windows and tabs from previous session on startup (default: true)\n",
                );
                content.push_str("# background_image: path to a background image file (PNG/JPEG/WebP); omit for no image\n");
                content.push_str("#   e.g. background_image = \"~/Pictures/bg.png\"\n");
                content.push_str("# background_image_opacity: opacity of the background image (0.0-1.0, default: 0.3)\n");
                content.push_str("# background_image_fit: how to fit the image: \"cover\" (default), \"contain\", \"fill\"\n");
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
                content.push_str("# right_click_action: action on right mouse button click (default: \"context_menu\")\n");
                content.push_str("#   \"context_menu\": show context menu\n");
                content.push_str("#   \"paste\": paste from clipboard\n");
                content.push_str("#   \"none\": do nothing\n");
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
#[path = "tests.rs"]
mod tests;
