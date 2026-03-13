# Quick Select オーバーレイ実装

## 概要

Phase 14.6 で実装された Quick Select モード。Cmd+Shift+Space で起動し、
画面上のパターン（URL、ファイルパス、git ハッシュ、数値）にヒントラベルを表示して
キーボード入力でクリップボードにコピーする。

## アーキテクチャ

### 状態管理 (`crates/sdit/src/app.rs`)

```rust
pub(crate) struct QuickSelectState {
    pub(crate) hints: Vec<QuickSelectHint>,
    pub(crate) input: String,  // ユーザーが入力中のヒント文字列
}

pub(crate) struct QuickSelectHint {
    pub(crate) label: String,   // "a", "s", "aa" など
    pub(crate) row: usize,
    pub(crate) start_col: usize,
    pub(crate) end_col: usize,
    pub(crate) text: String,
}
```

`SditApp.quick_select: Option<QuickSelectState>` がモードの on/off を管理する。

### ヒントラベル生成

```rust
const CHARS: &[u8] = b"asdfghjklqwertyuiopzxcvbnm";
// 0..25 → a,s,d,f,...  26以上 → aa,as,ad,...
```

Vim-like な home row 優先配置（WezTerm 方式を参考）。

### パターンマッチング (`crates/sdit-core/src/terminal/url_detector.rs`)

```rust
pub fn default_quick_select_patterns() -> Vec<Regex> {
    vec![
        Regex::new(r#"https?://..."#),        // URL
        Regex::new(r"/[^\s]+"),               // Unix ファイルパス
        Regex::new(r"\b[0-9a-fA-F]{7,40}\b"),// git ハッシュ
        Regex::new(r"\b\d+(?:\.\d+){0,3}(?::\d+)?\b"), // 数値・IP
    ]
}

pub fn detect_patterns_in_line(cells: &[Cell], patterns: &[Regex]) -> Vec<PatternMatch>
```

重複排除: 先のパターン（URL）が優先。`https://example.com/path` が URL と
ファイルパスの両方にマッチする場合は URL のみが採用される。

### オーバーレイ描画 (`crates/sdit/src/render.rs`)

`CellVertex::overwrite_cell` を使って既存セルを上書き描画:
- マッチ範囲全体: 青緑系背景 `[0.1, 0.5, 0.8, 1.0]`
- ヒントラベル部分: 濃い黄色背景 `[0.8, 0.6, 0.0, 1.0]`、白文字

### キー処理 (`crates/sdit/src/quick_select.rs`)

- **Escape**: モード終了
- **文字キー**: ヒント入力 → 前方一致で候補絞り込み → 完全一致でコピー+終了
- **Cmd/Ctrl 修飾**: 無視して消費（ショートカットが通常処理に届かないようにする）
- **Cmd+Shift+Space**: トグル動作（起動中なら終了）

### 設定 (`crates/sdit-core/src/config/mod.rs`)

```toml
[quick_select]
patterns = ["FOO-\\d+"]  # 追加パターン（デフォルトに append）
```

`config.quick_select.patterns` → `Regex::new()` でコンパイル → `all_patterns` に extend

## テスト

`cargo test --package sdit-core terminal::url_detector` で16テスト:
- `detect_patterns_finds_file_paths`
- `detect_patterns_finds_git_hashes`
- `detect_patterns_finds_urls_and_paths`
- `detect_patterns_no_overlap`
- `detect_patterns_custom_patterns`
- `detect_patterns_sorted_by_col`
- `detect_patterns_empty_cells`

## 注意点

- `quick_select` フィールドは `app.rs` の `SditApp` に直接格納（セッション単位ではなくウィンドウ単位）
- Quick Select モード中は全キー入力を消費する（PTY への送信がブロックされる）
- パターンがなければモードは起動しない（`hints.is_empty()` チェック）
- `overwrite_cell` は wgpu バッファの特定インデックスのみを書き換える。全グリッド再構築不要

## 関連ファイル

- `crates/sdit/src/quick_select.rs` — キー処理・モード起動
- `crates/sdit/src/app.rs` — `QuickSelectState`, `QuickSelectHint`, `generate_label`
- `crates/sdit/src/render.rs` — オーバーレイ描画（`redraw_session` 内）
- `crates/sdit-core/src/terminal/url_detector.rs` — `detect_patterns_in_line`, `default_quick_select_patterns`
- `crates/sdit-core/src/config/mod.rs` — `QuickSelectConfig`
