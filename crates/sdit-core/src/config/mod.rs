pub mod color;
pub mod font;
pub mod keybinds;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use self::color::ColorConfig;
use self::font::FontConfig;
use self::keybinds::{GlobalHotkeyBinding, KeybindConfig};
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

/// ウィンドウパディング領域の背景色設定。
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PaddingColor {
    /// ターミナル背景色と同じ色を使用する（デフォルト）。
    #[default]
    Background,
}

/// ウィンドウタイトルバーのサブタイトル表示設定。
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WindowSubtitle {
    /// サブタイトルを表示しない（デフォルト）。
    #[default]
    None,
    /// 現在の作業ディレクトリを表示する（OSC 7 で更新）。
    WorkingDirectory,
    /// セッション名を表示する。
    SessionName,
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

/// ウィンドウ色空間。macOS の wide color display（Display P3）に対応する。
///
/// Display P3 では `Bgra8UnormSrgb` サーフェスフォーマットを優先する。
/// 完全な P3 対応には Metal API が必要なため、現実装は sRGB より広色域に近づく近似。
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WindowColorspace {
    /// sRGB（デフォルト）。
    #[default]
    Srgb,
    /// Display P3 を優先。macOS の wide color display で有効。
    DisplayP3,
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
    /// 初期ワーキングディレクトリ（省略時: ホームディレクトリ）。
    ///
    /// `~` で始まる場合はホームディレクトリに展開する。
    /// `inherit_working_directory` より優先される。
    pub working_directory: Option<String>,
    /// パディング領域の背景色（デフォルト: "background" = ターミナル背景色と同じ）。
    #[serde(default)]
    pub padding_color: PaddingColor,
    /// ウィンドウタイトルバーのサブタイトル表示（デフォルト: "none"）。
    ///
    /// `"none"`: サブタイトルなし。
    /// `"working-directory"`: 現在の作業ディレクトリを表示（OSC 7 で更新）。
    /// `"session-name"`: セッション名を表示。
    #[serde(default)]
    pub subtitle: WindowSubtitle,
    /// 初期ウィンドウ位置 X（物理ピクセル）。省略時: OS 任せ。
    pub position_x: Option<i32>,
    /// 初期ウィンドウ位置 Y（物理ピクセル）。省略時: OS 任せ。
    pub position_y: Option<i32>,
    /// ウィンドウリサイズをセル整数倍にスナップする（デフォルト: false）。
    ///
    /// `true`: macOS/X11 がセルサイズの整数倍でのみリサイズするようヒントを設定する。
    /// ウィンドウ端に半端な余白が生まれず、グリッドが常に画面にぴったり収まる。
    #[serde(default)]
    pub resize_increments: bool,
    /// ウィンドウの色空間（デフォルト: "srgb"）。macOS の wide color display で有効。
    ///
    /// `"srgb"`: 標準 sRGB（デフォルト）。
    /// `"display-p3"`: Display P3 を優先。`Bgra8UnormSrgb` フォーマットが利用可能な場合に使用。
    #[serde(default)]
    pub colorspace: WindowColorspace,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            blur: false,
            padding_x: 0,
            padding_y: 0,
            columns: Self::DEFAULT_COLUMNS,
            rows: Self::DEFAULT_ROWS,
            startup_mode: StartupMode::Windowed,
            inherit_working_directory: true,
            confirm_close: ConfirmClose::ProcessRunning,
            decorations: Decorations::Full,
            always_on_top: false,
            restore_session: true,
            background_image: None,
            background_image_opacity: Self::DEFAULT_BACKGROUND_IMAGE_OPACITY,
            background_image_fit: BackgroundImageFit::Cover,
            working_directory: None,
            padding_color: PaddingColor::Background,
            subtitle: WindowSubtitle::None,
            position_x: None,
            position_y: None,
            resize_increments: false,
            colorspace: WindowColorspace::Srgb,
        }
    }
}

impl WindowConfig {
    /// 初期ウィンドウ幅のデフォルト値（列数）。
    pub const DEFAULT_COLUMNS: u16 = 80;
    /// 初期ウィンドウ高さのデフォルト値（行数）。
    pub const DEFAULT_ROWS: u16 = 24;
    /// ウィンドウ幅の最小値（列数）。
    pub const MIN_COLUMNS: u16 = 10;
    /// ウィンドウ幅の最大値（列数）。
    pub const MAX_COLUMNS: u16 = 500;
    /// ウィンドウ高さの最小値（行数）。
    pub const MIN_ROWS: u16 = 2;
    /// ウィンドウ高さの最大値（行数）。
    pub const MAX_ROWS: u16 = 200;
    /// パディングの最大値（ピクセル）。
    pub const MAX_PADDING: u16 = 200;
    /// 背景画像不透明度のデフォルト値。
    pub const DEFAULT_BACKGROUND_IMAGE_OPACITY: f32 = 0.3;
    /// ウィンドウ位置の最小値（物理ピクセル）。
    pub const MIN_POSITION: i32 = -16000;
    /// ウィンドウ位置の最大値（物理ピクセル）。
    pub const MAX_POSITION: i32 = 32000;

    /// opacity を安全な範囲にクランプする。
    ///
    /// NaN や Inf が渡された場合はデフォルト値 1.0 を返す。
    pub fn clamped_opacity(&self) -> f32 {
        if self.opacity.is_finite() { self.opacity.clamp(0.0, 1.0) } else { 1.0 }
    }

    /// `padding_x` を安全な範囲（0〜MAX_PADDING ピクセル）にクランプする。
    pub fn clamped_padding_x(&self) -> u16 {
        self.padding_x.min(Self::MAX_PADDING)
    }

    /// `padding_y` を安全な範囲（0〜MAX_PADDING ピクセル）にクランプする。
    pub fn clamped_padding_y(&self) -> u16 {
        self.padding_y.min(Self::MAX_PADDING)
    }

    /// `columns` を安全な範囲（MIN_COLUMNS〜MAX_COLUMNS）にクランプする。
    pub fn clamped_columns(&self) -> u16 {
        self.columns.clamp(Self::MIN_COLUMNS, Self::MAX_COLUMNS)
    }

    /// `rows` を安全な範囲（MIN_ROWS〜MAX_ROWS）にクランプする。
    pub fn clamped_rows(&self) -> u16 {
        self.rows.clamp(Self::MIN_ROWS, Self::MAX_ROWS)
    }

    /// `background_image_opacity` を安全な範囲にクランプする。
    ///
    /// NaN や Inf が渡された場合はデフォルト値 DEFAULT_BACKGROUND_IMAGE_OPACITY を返す。
    pub fn clamped_background_image_opacity(&self) -> f32 {
        if self.background_image_opacity.is_finite() {
            self.background_image_opacity.clamp(0.0, 1.0)
        } else {
            Self::DEFAULT_BACKGROUND_IMAGE_OPACITY
        }
    }

    /// `position_x/y` を安全な範囲にクランプして返す。
    ///
    /// 両方 `Some` のときのみ座標を返す。
    /// マルチディスプレイ環境を考慮し、合理的な範囲（MIN_POSITION〜MAX_POSITION）にクランプする。
    /// 一般的な最大解像度（8K × 4 ディスプレイ程度）を上限とする。
    pub fn clamped_position(&self) -> Option<(i32, i32)> {
        match (self.position_x, self.position_y) {
            (Some(x), Some(y)) => Some((
                x.clamp(Self::MIN_POSITION, Self::MAX_POSITION),
                y.clamp(Self::MIN_POSITION, Self::MAX_POSITION),
            )),
            _ => None,
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

impl BellConfig {
    /// ビジュアルベルのフェードアウト時間のデフォルト値（ミリ秒）。
    pub const DEFAULT_DURATION_MS: u32 = 150;
    /// duration_ms の最小値。
    pub const MIN_DURATION_MS: u32 = 1;
    /// duration_ms の最大値。
    pub const MAX_DURATION_MS: u32 = 5000;

    /// duration_ms を安全な範囲にクランプする（0 除算防止 + 長期ループ防止）。
    pub fn clamped_duration_ms(&self) -> u32 {
        self.duration_ms.clamp(Self::MIN_DURATION_MS, Self::MAX_DURATION_MS)
    }
}

impl Default for BellConfig {
    fn default() -> Self {
        Self { visual: true, dock_bounce: true, duration_ms: Self::DEFAULT_DURATION_MS }
    }
}

/// コマンド終了通知モード。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandNotifyMode {
    /// コマンド終了通知を送らない。
    Never,
    /// ウィンドウがフォーカスされていない場合のみ通知する（デフォルト）。
    Unfocused,
    /// 常に通知する。
    Always,
}

impl Default for CommandNotifyMode {
    fn default() -> Self {
        Self::Unfocused
    }
}

/// デスクトップ通知設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct NotificationConfig {
    /// デスクトップ通知を有効にする。
    pub enabled: bool,
    /// コマンド終了通知モード: "never" / "unfocused" / "always"（デフォルト: "unfocused"）。
    pub command_notify: CommandNotifyMode,
    /// コマンド終了通知の閾値（秒）。この時間以上かかったコマンドのみ通知する（デフォルト: 10）。
    pub command_notify_threshold: u32,
}

impl NotificationConfig {
    /// コマンド終了通知閾値のデフォルト値（秒）。
    pub const DEFAULT_COMMAND_NOTIFY_THRESHOLD: u32 = 10;
    /// コマンド終了通知閾値の最小値（秒）。
    pub const MIN_COMMAND_NOTIFY_THRESHOLD: u32 = 1;
    /// コマンド終了通知閾値の最大値（秒）。
    pub const MAX_COMMAND_NOTIFY_THRESHOLD: u32 = 3600;

    /// `command_notify_threshold` を安全な範囲にクランプする（MIN〜MAX秒）。
    pub fn clamped_command_notify_threshold(&self) -> u32 {
        self.command_notify_threshold
            .clamp(Self::MIN_COMMAND_NOTIFY_THRESHOLD, Self::MAX_COMMAND_NOTIFY_THRESHOLD)
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            command_notify: CommandNotifyMode::default(),
            command_notify_threshold: Self::DEFAULT_COMMAND_NOTIFY_THRESHOLD,
        }
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

impl ScrollingConfig {
    /// スクロール倍率のデフォルト値。
    pub const DEFAULT_MULTIPLIER: u32 = 3;
    /// スクロール倍率の最小値。
    pub const MIN_MULTIPLIER: u32 = 1;
    /// スクロール倍率の最大値。
    pub const MAX_MULTIPLIER: u32 = 100;

    /// multiplier を安全な範囲（MIN_MULTIPLIER〜MAX_MULTIPLIER）にクランプする。
    pub fn clamped_multiplier(&self) -> u32 {
        self.multiplier.clamp(Self::MIN_MULTIPLIER, Self::MAX_MULTIPLIER)
    }
}

impl Default for ScrollingConfig {
    fn default() -> Self {
        Self {
            multiplier: Self::DEFAULT_MULTIPLIER,
            scroll_to_bottom_on_keystroke: true,
            scroll_to_bottom_on_output: false,
        }
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
    /// word_chars の最大文字数。
    pub const MAX_WORD_CHARS: usize = 256;
    /// clipboard_codepoint_map の最大エントリ数。
    pub const MAX_CLIPBOARD_CODEPOINT_MAP_ENTRIES: usize = 64;
    /// codepoint_map の各 replacement 文字列の最大文字数。
    pub const MAX_REPLACEMENT_CHARS: usize = 256;
    /// codepoint_map 適用時の出力サイズ上限（入力の何倍まで許容するか）。
    pub const MAX_OUTPUT_MULTIPLIER: usize = 10;

    /// word_chars を最大 MAX_WORD_CHARS 文字にクランプして返す。
    pub fn clamped_word_chars(&self) -> &str {
        let s = self.word_chars.as_str();
        // バイト境界ではなく char 境界でカット
        let end = s.char_indices().nth(Self::MAX_WORD_CHARS).map(|(i, _)| i).unwrap_or(s.len());
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

        // キーをパースしてレンジリストを構築（最大 MAX_CLIPBOARD_CODEPOINT_MAP_ENTRIES エントリ）
        // replacement が長すぎるエントリは除外して warn する。
        let ranges: Vec<(u32, u32, &str)> = self
            .clipboard_codepoint_map
            .iter()
            .take(Self::MAX_CLIPBOARD_CODEPOINT_MAP_ENTRIES)
            .filter_map(|(range_str, replacement)| {
                if replacement.chars().count() > Self::MAX_REPLACEMENT_CHARS {
                    log::warn!(
                        "apply_codepoint_map: replacement for \"{}\" exceeds {} chars, skipping",
                        range_str,
                        Self::MAX_REPLACEMENT_CHARS
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

        let max_output_len = text.len().saturating_mul(Self::MAX_OUTPUT_MULTIPLIER);
        let mut result = String::with_capacity(text.len());
        for c in text.chars() {
            // 出力膨張チェック
            if result.len() > max_output_len {
                log::warn!(
                    "apply_codepoint_map: output exceeded {}x input size, truncating",
                    Self::MAX_OUTPUT_MULTIPLIER
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

fn default_click_repeat_interval() -> u32 {
    MouseConfig::DEFAULT_CLICK_REPEAT_INTERVAL
}

/// マウス設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct MouseConfig {
    /// タイピング中にマウスカーソルを非表示にする（デフォルト: false）。
    pub hide_when_typing: bool,
    /// 右クリック時の動作: "context_menu" (デフォルト), "paste", "none"。
    pub right_click_action: RightClickAction,
    /// ダブル/トリプルクリック判定の時間間隔（ミリ秒、デフォルト: 300、範囲: 50-2000）。
    #[serde(default = "default_click_repeat_interval")]
    pub click_repeat_interval: u32,
    /// マウスがウィンドウに乗ったとき自動フォーカスする（デフォルト: false）。
    pub focus_follows_mouse: bool,
    /// フォーカス取得時のマウスクリックを抑制する（デフォルト: false）。
    ///
    /// `true`: フォーカスされていないウィンドウをクリックしたとき、
    /// そのクリックはフォーカス取得のみに使われ、ターミナルへの入力として扱われない。
    #[serde(default)]
    pub swallow_mouse_click_on_focus: bool,
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

impl ScrollbarConfig {
    /// スクロールバー幅のデフォルト値（ピクセル）。
    pub const DEFAULT_WIDTH: u8 = 8;
    /// スクロールバー幅の最小値（ピクセル）。
    pub const MIN_WIDTH: u8 = 2;
    /// スクロールバー幅の最大値（ピクセル）。
    pub const MAX_WIDTH: u8 = 32;

    /// width を安全な範囲（2〜32）にクランプする。
    pub fn clamped_width(&self) -> u8 {
        self.width.clamp(Self::MIN_WIDTH, Self::MAX_WIDTH)
    }
}

impl Default for ScrollbarConfig {
    fn default() -> Self {
        Self { enabled: true, width: Self::DEFAULT_WIDTH }
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

impl MouseConfig {
    /// クリック繰り返し間隔のデフォルト値（ミリ秒）。
    pub const DEFAULT_CLICK_REPEAT_INTERVAL: u32 = 300;
    /// クリック繰り返し間隔の最小値（ミリ秒）。
    pub const MIN_CLICK_REPEAT_INTERVAL: u32 = 50;
    /// クリック繰り返し間隔の最大値（ミリ秒）。
    pub const MAX_CLICK_REPEAT_INTERVAL: u32 = 2000;

    /// `click_repeat_interval` を安全な範囲（MIN_CLICK_REPEAT_INTERVAL〜MAX_CLICK_REPEAT_INTERVAL ミリ秒）にクランプする。
    pub fn clamped_click_repeat_interval(&self) -> u32 {
        self.click_repeat_interval
            .clamp(Self::MIN_CLICK_REPEAT_INTERVAL, Self::MAX_CLICK_REPEAT_INTERVAL)
    }
}

impl Default for MouseConfig {
    fn default() -> Self {
        Self {
            hide_when_typing: false,
            right_click_action: RightClickAction::ContextMenu,
            click_repeat_interval: default_click_repeat_interval(),
            focus_follows_mouse: false,
            swallow_mouse_click_on_focus: false,
        }
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

impl ScrollbackConfig {
    /// スクロールバック行数のデフォルト値。
    pub const DEFAULT_LINES: u32 = 10_000;
    /// スクロールバック行数の最大値。
    pub const MAX_LINES: usize = 1_000_000;

    /// lines を安全な範囲にクランプする（0〜MAX_LINES）。
    pub fn clamped_lines(&self) -> usize {
        (self.lines as usize).min(Self::MAX_LINES)
    }
}

impl Default for ScrollbackConfig {
    fn default() -> Self {
        Self { lines: Self::DEFAULT_LINES }
    }
}

/// Quick Terminal のドロップダウン位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum QuickTerminalPosition {
    /// 画面上端からスライドイン（デフォルト）。
    Top,
    /// 画面下端からスライドイン。
    Bottom,
    /// 画面左端からスライドイン。
    Left,
    /// 画面右端からスライドイン。
    Right,
}

impl Default for QuickTerminalPosition {
    fn default() -> Self {
        Self::Top
    }
}

fn default_quick_terminal_size() -> f32 {
    QuickTerminalConfig::DEFAULT_SIZE
}

fn default_quick_terminal_hotkey() -> String {
    "ctrl+`".to_owned()
}

fn default_quick_terminal_animation_duration() -> f32 {
    QuickTerminalConfig::DEFAULT_ANIMATION_DURATION
}

/// Quick Terminal（ドロップダウンターミナル）設定。
///
/// グローバルホットキーで画面端からスライドインするターミナルウィンドウを制御する。
/// macOS 固有機能。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct QuickTerminalConfig {
    /// Quick Terminal を有効にする（デフォルト: false）。
    pub enabled: bool,
    /// ドロップダウンの出現位置: "top" (デフォルト), "bottom", "left", "right"。
    #[serde(default)]
    pub position: QuickTerminalPosition,
    /// ウィンドウが占める画面比率（0.1〜1.0、デフォルト: 0.4）。
    #[serde(default = "default_quick_terminal_size")]
    pub size: f32,
    /// グローバルホットキー文字列（デフォルト: "ctrl+`"）。
    ///
    /// 形式: "ctrl+`", "ctrl+shift+t" 等。
    #[serde(default = "default_quick_terminal_hotkey")]
    pub hotkey: String,
    /// スライドイン/アウトのアニメーション時間（秒、デフォルト: 0.2）。
    #[serde(default = "default_quick_terminal_animation_duration")]
    pub animation_duration: f32,
}

impl QuickTerminalConfig {
    /// ウィンドウサイズ比率のデフォルト値。
    pub const DEFAULT_SIZE: f32 = 0.4;
    /// ウィンドウサイズ比率の最小値。
    pub const MIN_SIZE: f32 = 0.1;
    /// ウィンドウサイズ比率の最大値。
    pub const MAX_SIZE: f32 = 1.0;
    /// アニメーション時間のデフォルト値（秒）。
    pub const DEFAULT_ANIMATION_DURATION: f32 = 0.2;
    /// アニメーション時間の最小値（秒）。
    pub const MIN_ANIMATION_DURATION: f32 = 0.0;
    /// アニメーション時間の最大値（秒）。
    pub const MAX_ANIMATION_DURATION: f32 = 2.0;

    /// size を安全な範囲（MIN_SIZE〜MAX_SIZE）にクランプする。
    ///
    /// NaN や Inf が渡された場合はデフォルト値 DEFAULT_SIZE を返す。
    pub fn clamped_size(&self) -> f32 {
        if self.size.is_finite() {
            self.size.clamp(Self::MIN_SIZE, Self::MAX_SIZE)
        } else {
            Self::DEFAULT_SIZE
        }
    }

    /// animation_duration を安全な範囲（MIN_ANIMATION_DURATION〜MAX_ANIMATION_DURATION）にクランプする。
    ///
    /// NaN や Inf が渡された場合はデフォルト値 DEFAULT_ANIMATION_DURATION を返す。
    pub fn clamped_animation_duration(&self) -> f32 {
        if self.animation_duration.is_finite() {
            self.animation_duration
                .clamp(Self::MIN_ANIMATION_DURATION, Self::MAX_ANIMATION_DURATION)
        } else {
            Self::DEFAULT_ANIMATION_DURATION
        }
    }
}

impl Default for QuickTerminalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            position: QuickTerminalPosition::Top,
            size: default_quick_terminal_size(),
            hotkey: default_quick_terminal_hotkey(),
            animation_duration: default_quick_terminal_animation_duration(),
        }
    }
}

/// カスタムリンク設定のバリデーション定数（後方互換のためモジュールレベルにも残す）。
const MAX_LINK_ENTRIES: usize = Config::MAX_LINK_ENTRIES;
const MAX_LINK_REGEX_LEN: usize = Config::MAX_LINK_REGEX_LEN;
const MAX_LINK_ACTION_LEN: usize = Config::MAX_LINK_ACTION_LEN;

/// OSC 10/11/12 カラー問い合わせの応答形式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OscColorReportFormat {
    /// 8-bit: rgb:RR/GG/BB
    #[serde(rename = "8-bit")]
    EightBit,
    /// 16-bit: rgb:RRRR/GGGG/BBBB（デフォルト）
    #[serde(rename = "16-bit")]
    SixteenBit,
}

impl Default for OscColorReportFormat {
    fn default() -> Self {
        Self::SixteenBit
    }
}

/// ターミナル設定。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct TerminalConfig {
    /// Grapheme 幅計算方式: "unicode" (デフォルト) または "legacy"。
    ///
    /// `"unicode"`: Unicode 標準の幅計算を使用する（デフォルト）。
    /// `"legacy"`: 旧来の wcswidth 互換モード（将来対応）。
    pub grapheme_width_method: GraphemeWidthMethod,
    /// 東アジア曖昧幅文字（○□★→℃ 等）のセル幅扱い。
    ///
    /// `"narrow"`: 1 セル幅（デフォルト。unicode-width の標準動作）。
    /// `"wide"`: 2 セル幅（CJK 環境向け。全角扱い）。
    pub east_asian_ambiguous_width: EastAsianAmbiguousWidth,
    /// OSC 10/11/12 カラー問い合わせの応答形式。
    ///
    /// `"8-bit"`: `rgb:RR/GG/BB` 形式で応答する。
    /// `"16-bit"`: `rgb:RRRR/GGGG/BBBB` 形式で応答する（デフォルト）。
    pub osc_color_report_format: OscColorReportFormat,
    /// ウィンドウタイトルの報告 (CSI 21 t) を許可するか。
    ///
    /// セキュリティ上の理由からデフォルトは `false`（拒否）。
    /// `true` に設定するとアプリケーションがタイトルを問い合わせできる。
    pub title_report: bool,
    /// ENQ (0x05) への応答文字列。
    ///
    /// `null` または未設定の場合は応答しない（デフォルト）。
    /// 最大 256 文字。
    pub enquiry_response: Option<String>,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            grapheme_width_method: GraphemeWidthMethod::Unicode,
            east_asian_ambiguous_width: EastAsianAmbiguousWidth::Narrow,
            osc_color_report_format: OscColorReportFormat::default(),
            title_report: false,
            enquiry_response: None,
        }
    }
}

impl TerminalConfig {
    /// `enquiry_response` を最大 256 文字にクランプして返す。
    ///
    /// ライブラリ利用時でも長さ制限が確実に適用されるよう、
    /// `sdit` バイナリ側の `clamp_enquiry_response` の代わりにこのメソッドを使用する。
    pub fn clamped_enquiry_response(&self) -> Option<String> {
        self.enquiry_response.as_ref().map(|s| {
            const MAX_ENQ_CHARS: usize = 256;
            if s.chars().count() > MAX_ENQ_CHARS {
                let idx = s.char_indices().nth(MAX_ENQ_CHARS).map(|(i, _)| i).unwrap_or(s.len());
                s[..idx].to_string()
            } else {
                s.clone()
            }
        })
    }
}

/// Grapheme 幅計算方式。
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum GraphemeWidthMethod {
    /// Unicode 標準の幅計算（デフォルト）。
    #[default]
    Unicode,
    /// 旧来の wcswidth 互換モード（将来対応）。
    Legacy,
}

/// 東アジア曖昧幅（East Asian Ambiguous Width）文字の幅扱い。
///
/// Unicode では○・□・★・→・℃ 等を「曖昧幅」として定義している。
/// CJK 環境では慣習的に 2 セル（全角）扱いすることが多い。
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum EastAsianAmbiguousWidth {
    /// 1 セル幅（unicode-width デフォルト）。
    #[default]
    Narrow,
    /// 2 セル幅（CJK 環境向け）。
    Wide,
}

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
    /// Quick Terminal 設定。
    pub quick_terminal: QuickTerminalConfig,
    /// カスタムリンク設定。最大 32 エントリ。
    ///
    /// ```toml
    /// [[links]]
    /// regex = "JIRA-\\d+"
    /// action = "open:https://jira.example.com/browse/$0"
    /// ```
    #[serde(default)]
    pub links: Vec<LinkConfig>,
    /// PTY 起動時に注入する追加環境変数（最大 64 エントリ）。
    ///
    /// 親プロセスの環境変数に追加・上書きする形で設定される。
    /// キー/値に制御文字が含まれるエントリはスキップして warn ログを出す。
    ///
    /// ```toml
    /// [env]
    /// TERM_PROGRAM = "sdit"
    /// COLORTERM = "truecolor"
    /// ```
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// ターミナル設定。
    pub terminal: TerminalConfig,
    /// グローバルホットキー設定（macOS のみ有効）。
    #[serde(default)]
    pub global_hotkeys: Vec<GlobalHotkeyBinding>,
}

impl Config {
    /// カスタムリンク設定の最大エントリ数。
    pub const MAX_LINK_ENTRIES: usize = 32;
    /// リンク正規表現パターンの最大バイト長。
    pub const MAX_LINK_REGEX_LEN: usize = 512;
    /// リンクアクション文字列の最大バイト長。
    pub const MAX_LINK_ACTION_LEN: usize = 1024;

    /// カスタムリンク設定を最大 MAX_LINK_ENTRIES 件・regex/action 文字列長を制限して返す。
    pub fn clamped_links(&self) -> impl Iterator<Item = &LinkConfig> {
        self.links.iter().take(Self::MAX_LINK_ENTRIES).filter(|lc| {
            lc.regex.len() <= Self::MAX_LINK_REGEX_LEN
                && lc.action.len() <= Self::MAX_LINK_ACTION_LEN
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
                content.push_str("# search_foreground: search highlight foreground color as hex \"#RRGGBB\"; omit to use default\n");
                content.push_str("# search_background: search highlight background color as hex \"#RRGGBB\"; omit to use default\n");
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
                content.push_str("# working_directory: initial working directory for new sessions (default: home directory)\n");
                content.push_str("#   e.g. working_directory = \"~/Projects\"\n");
                content.push_str("# padding_color: color of the padding area (default: \"background\" = same as terminal background)\n");
                content.push_str("# subtitle: window subtitle display (default: \"none\")\n");
                content.push_str("#   \"none\": no subtitle\n");
                content.push_str("#   \"working-directory\": show current working directory (updated via OSC 7)\n");
                content.push_str("#   \"session-name\": show session name\n");
                content.push_str("# resize_increments: snap window resize to cell size multiples (default: false)\n");
                content.push_str("#   true: window resizes in exact cell-width/cell-height steps (no fractional padding)\n");
                content.push_str("# colorspace: window color space (default: \"srgb\"); macOS wide color display support\n");
                content.push_str("#   \"srgb\": standard sRGB\n");
                content.push_str("#   \"display-p3\": prefer Display P3 (uses Bgra8UnormSrgb format when available)\n");
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
                content.push_str("# click_repeat_interval: double/triple click detection interval in milliseconds (50-2000, default: 300)\n");
                content.push_str("# swallow_mouse_click_on_focus: absorb click when window gains focus (default: false)\n");
                content
                    .push_str("#   true: click only focuses the window, not passed to terminal\n");
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
            } else if line == "[quick_terminal]" {
                content.push('\n');
                content.push_str("# ── Quick Terminal ─────────────────────────────────────────\n");
                content.push_str(
                    "# enabled: enable the Quick Terminal dropdown (default: false, macOS only)\n",
                );
                content.push_str("# position: slide-in direction: \"top\" (default), \"bottom\", \"left\", \"right\"\n");
                content
                    .push_str("# size: fraction of screen width/height (0.1-1.0, default: 0.4)\n");
                content.push_str("# hotkey: global hotkey string (default: \"ctrl+`\")\n");
                content.push_str("# animation_duration: slide-in/out animation in seconds (0.0-2.0, default: 0.2)\n");
            } else if line == "[[global_hotkeys]]" {
                content.push('\n');
                content.push_str("# ── Global Hotkeys ─────────────────────────────────────────\n");
                content.push_str(
                    "# System-wide hotkeys (macOS only, requires Accessibility permission).\n",
                );
                content.push_str("# hotkey: key combination e.g. \"cmd+shift+alt+t\"\n");
                content.push_str("# action: action name e.g. \"BringToFront\", \"NewWindow\"\n");
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
            } else if line == "[env]" {
                content.push('\n');
                content.push_str("# ── Environment Variables ─────────────────────────────────\n");
                content.push_str("# Extra environment variables injected into each PTY session (max 64 entries).\n");
                content.push_str(
                    "# Keys/values containing control characters are skipped with a warning.\n",
                );
                content.push_str("# Example:\n");
                content.push_str("#   [env]\n");
                content.push_str("#   TERM_PROGRAM = \"sdit\"\n");
                content.push_str("#   COLORTERM = \"truecolor\"\n");
            } else if line == "[terminal]" {
                content.push('\n');
                content.push_str("# ── Terminal ───────────────────────────────────────────────\n");
                content.push_str("# grapheme_width_method: grapheme width calculation method (default: \"unicode\")\n");
                content.push_str("#   \"unicode\": standard Unicode width calculation\n");
                content
                    .push_str("#   \"legacy\": legacy wcswidth-compatible mode (future support)\n");
                content.push_str("# east_asian_ambiguous_width: cell width for East Asian Ambiguous characters e.g. ○□★ (default: \"narrow\")\n");
                content.push_str("#   \"narrow\": 1 cell (standard unicode-width behavior)\n");
                content.push_str("#   \"wide\": 2 cells (CJK environment preference)\n");
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
