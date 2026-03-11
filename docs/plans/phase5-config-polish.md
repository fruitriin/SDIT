# Phase 5 — 設定・仕上げ

## 目的

TOML設定ファイル、フォント設定（日本語対応含む）、カラーテーマ設定、セッション永続化など、
日常使いに必要な機能を整備する。

## 前提条件

- Phase 4 の縦タブ実装が完了していること

## サブフェーズ一覧

| サブフェーズ | 名称 | 依存 |
|---|---|---|
| 5.1 | TOML設定基盤 + フォント設定 | なし |
| 5.2 | カラーテーマ設定 | 5.1 |
| 5.3 | 日本語フォント対応（CJKフォールバック） | 5.1 |
| 5.4 | セッション永続化 | 5.1 |
| 5.5 | 統合テスト強化（コントラスト・文字化け） | 5.2, 5.3 |

---

## Phase 5.1 — TOML設定基盤 + フォント設定

**変更ファイル:** `sdit-config/src/lib.rs`, `sdit-config/src/font.rs`(新規), `sdit-render/src/font.rs`, `main.rs`

TOML スキーマ:
```toml
[font]
family = "Menlo"        # macOS デフォルト
size = 14.0
line_height = 1.2
```

- `Config` struct に `font: FontConfig` フィールド
- `Config::load(path)` — ファイル不在時はデフォルト値
- 設定パス: `~/.config/sdit/sdit.toml`
- `FontContext::new` を設定値で初期化
- [x] 完了

## Phase 5.2 — カラーテーマ設定

**変更ファイル:** `sdit-config/src/color.rs`(新規), `sdit-render/src/color.rs`(新規), `main.rs`

```toml
[colors]
theme = "catppuccin-mocha"  # | "catppuccin-latte" | "gruvbox-dark" | "custom"
```

- 組み込みテーマ 3種 + カスタム RGB
- `ResolvedColors` struct でレンダー用 f32 RGBA
- `wcag_contrast_ratio()` ヘルパー
- ハードコード色を設定参照に置換
- [x] 完了

## Phase 5.3 — 日本語フォント対応（CJKフォールバック）

**変更ファイル:** `sdit-config/src/font.rs`, `sdit-render/src/font.rs`, `sdit-render/src/pipeline.rs`

```toml
[font]
fallback_families = ["Hiragino Sans", "Noto Sans CJK JP"]
```

- cosmic-text FontSystem に CJK フォールバックフォントを登録
- `WIDE_CHAR` フラグによる 2倍幅描画
- `WIDE_CHAR_SPACER` セルの描画スキップ
- [x] 完了

## Phase 5.4 — セッション永続化

**変更ファイル:** `sdit-session/src/persistence.rs`(新規), `sdit-session/src/session.rs`, `main.rs`

```toml
[session]
restore_on_startup = true
save_on_exit = true
```

- `AppSnapshot` (windows → sessions → cwd) を TOML で保存/復元
- 保存先: `~/.local/state/sdit/session.toml`
- PTY 内容は保存しない（cwd のみ）
- スナップショット破損時はデフォルト起動
- [x] 完了

## Phase 5.5 — 統合テスト強化（コントラスト・文字化け）

**変更ファイル:** `sdit-render/tests/contrast_test.rs`(新規), `sdit-core/tests/cjk_render_test.rs`(新規)

- WCAG AA 基準のコントラスト比テスト（通常 4.5:1、dim 3:0:1）
- CJK 文字の unicode_width テスト
- 日本語グリフのラスタライズ成否テスト
- [x] 完了

## 依存クレート追加

| クレート | 追加先 | 用途 |
|---|---|---|
| `serde = { version = "1", features = ["derive"] }` | `sdit-config`, `sdit-session` | シリアライズ |
| `toml = "0.8"` | `sdit-config`, `sdit-session` | TOML パース |
| `dirs = "5"` | `sdit-config`, `sdit-session` | XDG パス解決 |

## セキュリティ考慮事項

- TOML デシリアライズ: 極端なフォントサイズ・パストラバーサル防御
- RGB カスタム色: `#RRGGBB` パースのパニック防止
- スナップショット: cwd のパストラバーサル、アトミック書き込み（一時ファイル + rename）

## 参照

- `refs/alacritty/alacritty-config/src/` — 設定スキーマ
- `refs/ghostty/src/font/` — フォントシェーピング
- `refs/zellij/zellij-server/src/` — セッション永続化

## 完了条件（Phase 5 全体）

- [x] `~/.config/sdit/sdit.toml` でフォント・テーマ設定が動作する
- [x] 設定ファイル不在時にデフォルト値で起動できる
- [x] 日本語文字が豆腐にならず表示される
- [x] 全角文字が 2 セル幅で描画される
- [x] カラーテーマを切り替えられる
- [x] セッション永続化（cwd 復元）が動作する
- [x] WCAG AA コントラスト基準のテストがパス
- [x] `cargo test` 全通過
- [x] セキュリティレビュー完了

## セキュリティレビュー結果

| ID | 重要度 | 内容 | 対応 |
|---|---|---|---|
| M-1 | Medium | `clamped_size()` が NaN/Infinity で防御無効 | 修正済み: `is_finite()` チェック追加 |
| M-2 | Medium | 永続化の一時ファイル名が予測可能（TOCTOU） | 修正済み: PID+ナノ秒を含む一時ファイル名 |
| M-3 | Medium | `cell_width_scale` の異常値で GPU 描画破綻 | 修正済み: シェーダー内で `clamp(1.0, 2.0)` |
| L-1 | Low | 設定ファイルの巨大ファイル読み込み DoS | 記録のみ |
| L-2 | Low | 永続化ファイルの巨大ファイル読み込み DoS | 記録のみ |
| L-3 | Low | `fallback_families` の無制限デシリアライゼーション | 記録のみ |
| L-4 | Low | `sessions` の無制限デシリアライゼーション | 記録のみ |
| L-5 | Low | `cwd` パストラバーサル | 記録のみ |
| I-1 | Info | テーマ名タイポ時のサイレントフォールバック | 記録のみ |
