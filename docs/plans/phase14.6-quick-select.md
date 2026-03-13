# Phase 14.6: Quick Select

**概要**: キーボードショートカットで画面上のテキストパターン（URL、パス、ハッシュ等）をハイライトし、ヒントキーで即座にコピーする WezTerm QuickSelect 風の機能。

**状態**: **完了**

## 実装結果

- `PatternMatch` 型 + `detect_patterns_in_line()` で URL/パス/ハッシュ/数値を検出（url_detector.rs）
- `QuickSelectState` + ヒントラベル生成 a-z, aa-az...（app.rs）
- `QuickSelectConfig { patterns }` + `clamped_patterns()` 上限50件（config/mod.rs）
- `Action::QuickSelect` キーバインド Cmd+Shift+Space（keybinds.rs）
- event_loop.rs + quick_select.rs: モード開始・ヒントキー入力・クリップボードコピー
- render.rs: オーバーレイ描画（ハイライト + ヒントラベル）
- テスト 10件追加

### セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| Medium | M-1 | カスタム正規表現を毎回コンパイル | コンパイルキャッシュ導入済み |
| Medium | M-2 | パターン数無制限 | clamped_patterns() 上限50件 |
| Low | L-1 | デフォルトパターン毎回コンパイル | OnceLock キャッシュ導入済み |
| Low | L-2 | PatternMatch.text 長さ制限なし | Plan に記録 |
| Low | L-3 | コピーテキスト全文ログ出力 | バイト数のみに修正済み |
| Info | I-1 | NUL セルのクリップボード混入 | Plan に記録 |
| Info | I-2 | overwrite_cell の境界チェック | Plan に記録 |

## 背景

- マウスなしでターミナル上のテキストを素早くコピーしたい需要がある
- WezTerm の QuickSelect、Ghostty の quick_select がこれを実現
- URL 検出（Phase 8.2）の基盤を拡張して実装可能

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| QuickSelectOverlay | オーバーレイ状態管理（マッチ一覧 + ヒントラベル割り当て） | sdit (`app.rs`) | **完了** |
| パターンマッチング | URL + ファイルパス + git ハッシュ + 数値をスクリーン全体からマッチ | sdit-core (`terminal/url_detector.rs` 拡張) | **完了** |
| ヒントラベル生成 | a-z の1-2文字ラベルをマッチに割り当て | sdit (`app.rs`) | **完了** |
| オーバーレイ描画 | マッチのハイライト + ヒントラベルの表示 | sdit (`render.rs`) | **完了** |
| キー入力処理 | ヒントキー入力 → クリップボードにコピー → オーバーレイ終了 | sdit (`quick_select.rs`) | **完了** |
| テスト | パターンマッチ 7件 + ヒントラベル割り当て 3件 | sdit-core, sdit | **完了** |

## 設定例

```toml
[quick_select]
# キーバインドは keybinds で設定: Cmd+Shift+Space = "QuickSelect"
patterns = []   # 追加の正規表現パターン（デフォルトパターンに追加）
```

## 参照

- `refs/wezterm/wezterm-gui/src/overlay/quickselect.rs` — QuickSelect 実装

## 依存関係

- Phase 8.2（URL 検出）— パターンマッチングの基盤
