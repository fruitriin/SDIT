# Phase 14.6: Quick Select

**概要**: キーボードショートカットで画面上のテキストパターン（URL、パス、ハッシュ等）をハイライトし、ヒントキーで即座にコピーする WezTerm QuickSelect 風の機能。

**状態**: 未着手

## 背景

- マウスなしでターミナル上のテキストを素早くコピーしたい需要がある
- WezTerm の QuickSelect、Ghostty の quick_select がこれを実現
- URL 検出（Phase 8.2）の基盤を拡張して実装可能

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| QuickSelectOverlay | オーバーレイ状態管理（マッチ一覧 + ヒントラベル割り当て） | sdit (`app.rs`) | 未着手 |
| パターンマッチング | URL + ファイルパス + git ハッシュ + 数値をスクリーン全体からマッチ | sdit-core (`terminal/url_detector.rs` 拡張) | 未着手 |
| ヒントラベル生成 | a-z の1-2文字ラベルをマッチに割り当て | sdit (`app.rs`) | 未着手 |
| オーバーレイ描画 | マッチのハイライト + ヒントラベルの表示 | sdit (`render.rs`) | 未着手 |
| キー入力処理 | ヒントキー入力 → クリップボードにコピー → オーバーレイ終了 | sdit (`input.rs`) | 未着手 |
| テスト | パターンマッチ 3件 + ヒントラベル割り当て 2件 | sdit-core, sdit | 未着手 |

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
