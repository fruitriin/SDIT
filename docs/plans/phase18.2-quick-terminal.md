# Phase 18.2: Quick Terminal

**概要**: グローバルホットキーで画面上端からスライドインするドロップダウンターミナルを実装する（macOS）。

**状態**: 完了

## アーキテクチャ方針

### unsafe_code 制約の解決

SDIT は `unsafe_code = "deny"` をワークスペース全体に適用しているため、CGEvent tap を直接使えない。
`core-graphics` クレートの安全ラッパーを活用し、unsafe が必要な最小限の部分のみ
`#[cfg(target_os = "macos")]` モジュール内で `#[allow(unsafe_code)]` を適用する。

### ウィンドウ管理

Ghostty は NSPanel（補助パネル）を使用しているが、winit は NSWindow のみサポート。
SDIT では以下の代替アプローチを採用する:

1. **通常の winit ウィンドウ**をボーダーレス + AlwaysOnTop で作成
2. `raw_window_handle` で macOS の `NSWindow` ハンドルを取得
3. `objc2` クレートで NSWindow の追加設定（`collectionBehavior` 等）を適用

### アニメーション

winit にはネイティブアニメーション機構がないため、フレームベースの位置補間で実装する:
- `request_redraw()` ループ内で `Instant` ベースのイージング関数を適用
- 0.2 秒のスライドイン/アウト

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| Config に quick_terminal 設定追加 | enabled, position, size, hotkey | sdit-core (`config/mod.rs`) | 完了 |
| QuickTerminal 状態管理 | 表示/非表示状態、アニメーション制御 | sdit (`quick_terminal.rs`) | 完了 |
| グローバルホットキー登録 | `global-hotkey` クレートで登録 | sdit (`quick_terminal.rs`) | 完了 |
| Quick Terminal ウィンドウ生成 | ボーダーレス + AlwaysOnTop + 指定位置 | sdit (`window_ops.rs`) | 完了 |
| スライドアニメーション | フレームベースの位置補間（Hermite easing） | sdit (`quick_terminal.rs`) | 完了 |
| 前アプリ復帰 | 初期実装では省略（winit の focus API で今後対応可能） | sdit (`quick_terminal.rs`) | 省略 |
| テスト | 設定デシリアライズ + 状態遷移テスト | sdit-core | 完了 |

## 実装メモ

- グローバルホットキーには `global-hotkey = "0.7"` クレートを採用（unsafe 不要）
- Accessibility パーミッションがない場合はログ警告を出して機能を無効化する
- アニメーションは `about_to_wait` から `tick_quick_terminal_animation()` を呼ぶフレームベース実装
- 前アプリ復帰（NSRunningApplication）は初期実装では省略。将来の改善タスクとして残す

## セキュリティメモ

- グローバルホットキー受信スレッドは EventLoopProxy 経由でイベントを送信するのみ（副作用なし）
- ウィンドウ生成はメインスレッドのみ（スレッドセーフ）
- Accessibility パーミッションの要求は macOS が自動管理する

## 設定例

```toml
[quick_terminal]
enabled = false
position = "top"      # top | bottom | left | right
size = 0.4            # 画面比率
hotkey = "ctrl+`"     # グローバルホットキー
animation_duration = 0.2  # 秒
```

## 依存クレート候補

- `core-graphics` — CGEvent tap の安全ラッパー（CGEventTapCreate 等）
- `objc2` + `objc2-app-kit` — NSWindow の追加設定（collectionBehavior）
- `objc2-foundation` — NSRunningApplication（前アプリ復帰）

## 参照

- `refs/ghostty/macos/Sources/Features/QuickTerminal/` — Quick Terminal 実装全体
- `refs/ghostty/macos/Sources/Features/Global Keybinds/GlobalEventTap.swift` — CGEvent tap
- `refs/ghostty/src/config/Config.zig` — quick-terminal-position, quick-terminal-size

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| High | H-1 | ホットキー受信スレッドの recv() エラーハンドリング不足 | 修正済み: Err で log::error + break、spawn 失敗時もログ |
| High | H-2 | ホットキー文字列パースの DoS（長さ・パーツ数無制限） | 修正済み: 256文字上限 + 8パーツ上限 + 空キー名チェック |
| High | H-3 | ウィンドウ位置計算の整数オーバーフロー | 修正済み: screen_size=0 フォールバック + size_ratio 二重 clamp + win サイズ制限 |
| Medium | M-4 | 画面サイズ取得失敗時のサイレント失敗 | 修正済み: ログ警告追加 |
| Medium | M-5 | ホットキー初期化失敗時の不明確なエラー | 修正済み: Accessibility パーミッション言及の警告ログ |
| Medium | M-6 | アニメーション完了判定の冗長コード | 修正済み: 重複コード整理 |
| Medium | M-7 | Config validate 未実装 | 記録のみ: clamped メソッドで安全に処理 |
| Low | L-1 | u32→i32 キャスト警告 | 記録のみ: 通常ディスプレイサイズでは問題なし |
| Low | L-2 | format! 文字列のインライン化 | 記録のみ: スタイルのみ |
| Info | I-1 | Accessibility パーミッション要求ダイアログ | 記録のみ: 将来対応 |

## 備考

- **Accessibility パーミッション**: CGEvent tap はシステム環境設定の「アクセシビリティ」許可が必要。未許可時は機能を無効化し、ログで通知する
- **entitlements**: Debug/Release の entitlements に必要に応じて追記
