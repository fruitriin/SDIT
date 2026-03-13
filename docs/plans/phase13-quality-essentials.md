# Phase 13: 当たり前品質向上

**概要**: 日常使用で「ないと乗り換えがつらい」機能を追加し、SDIT を常用可能な品質にする。

**状態**: 未着手

## Phase 13.1: macOS Option as Alt（優先度: 高、複雑度: 低）

macOS の Option キーを Alt として扱う設定。これがないと readline ショートカット（Alt+B/F/D 等）が使えない。

| タスク | 詳細 | 変更先 |
|---|---|---|
| 設定項目追加 | `option_as_alt: Both/Left/Right/None` | sdit-core (`config/`) |
| winit 連携 | `Window::set_option_as_alt()` でフラグ反映 | sdit (`event_loop.rs`) |
| Hot Reload 対応 | 設定変更時にリアルタイム反映 | sdit (`config_watcher`) |

参照: `refs/alacritty/alacritty/src/config/window.rs`, `refs/ghostty/src/input/keyboard.zig`

## Phase 13.2: ビジュアルベル + システム通知（優先度: 中高、複雑度: 低）

BEL (0x07) 受信時の視覚フィードバック。現在はログ出力のみ。

| タスク | 詳細 | 変更先 |
|---|---|---|
| ビジュアルベル | 画面を一瞬フラッシュ（背景色反転→フェード） | sdit (`render.rs`) |
| macOS Dock バウンス | `NSApp::requestUserAttention()` で通知 | sdit (`event_loop.rs`) |
| 設定項目 | `bell.visual = true/false`, `bell.dock_bounce = true/false` | sdit-core (`config/`) |

参照: `refs/alacritty/alacritty/src/display/bell.rs`

## Phase 13.3: 背景透過 + macOS blur（優先度: 中高、複雑度: 中）

ウィンドウ背景の不透明度設定。macOS ユーザーに人気の高い視覚カスタマイズ。

| タスク | 詳細 | 変更先 |
|---|---|---|
| 背景アルファ設定 | `window.opacity: 0.0〜1.0` | sdit-core (`config/`) |
| wgpu クリアカラー | アルファチャンネルを反映 | sdit-core (`render/pipeline.rs`) |
| macOS blur | `NSVisualEffectView` 連携（winit raw handle 経由） | sdit (`render.rs`) |

参照: `refs/alacritty/alacritty/src/config/window.rs`

## Phase 13.4: Unsafe Paste 警告（優先度: 中高、複雑度: 低〜中）

改行を含むペースト時に確認ダイアログを表示するセキュリティ機能。

| タスク | 詳細 | 変更先 |
|---|---|---|
| ペースト安全性判定 | 改行・制御文字を含むテキストの検出 | sdit (`event_loop.rs`) |
| 確認ダイアログ | macOS NSAlert で確認 | sdit (`event_loop.rs`) |
| 設定項目 | `paste.confirm_unsafe = true/false` | sdit-core (`config/`) |

参照: `refs/ghostty/src/input/paste.zig`

## Phase 13.5: Kitty Keyboard Protocol（優先度: 中高、複雑度: 中）

neovim 等が要求する拡張キーボードプロトコル。修飾キーの正確な報告、キーリリースイベント等を提供。

| タスク | 詳細 | 変更先 |
|---|---|---|
| CSI u エンコーディング | Kitty progressive enhancement flags 対応 | sdit (`input.rs`) |
| モード管理 | `CSI > Ps u` (push) / `CSI < u` (pop) | sdit-core (`terminal/`) |
| modifyOtherKeys | xterm mode 2 互換 | sdit (`input.rs`) |

参照: `refs/ghostty/src/input/key_encode.zig`, `refs/ghostty/src/input/kitty.zig`

## Phase 13.6: デスクトップ通知（優先度: 中、複雑度: 低〜中）

OSC 9 / OSC 99 でシステム通知を発行。長時間コマンド完了通知に有用。

| タスク | 詳細 | 変更先 |
|---|---|---|
| OSC 9/99 パース | 通知タイトル・本文の抽出 | sdit-core (`terminal/`) |
| macOS 通知連携 | `UNUserNotificationCenter` API | sdit (`event_loop.rs`) |
| 設定項目 | `notification.enabled = true/false` | sdit-core (`config/`) |

## 依存関係

なし（各サブフェーズは独立して実装可能）

## 実装順序の推奨

1. Phase 13.1（Option as Alt）— 最も低コスト・高インパクト
2. Phase 13.2（ビジュアルベル）— 低コスト
3. Phase 13.4（Unsafe Paste 警告）— セキュリティ
4. Phase 13.3（背景透過）— ユーザー人気
5. Phase 13.5（Kitty Keyboard Protocol）— neovim ユーザー向け
6. Phase 13.6（デスクトップ通知）— 利便性
