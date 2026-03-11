# Phase 2.5 — 統合テスト基盤

## 目的
Phase 2 で完成したバイナリの品質を保証する統合テスト基盤を構築する。
ヘッドレステスト（PTY パイプライン）と GUI テスト（画面キャプチャ・操作）の
2層で退行を検知できる体制を作る。

## 前提条件
- Phase 2 の単一ウィンドウ表示 + PTY 接続が完了していること

## 調査結果

### GUI テストツール評価

| ツール | 用途 | SDIT 適用性 | 備考 |
|---|---|---|---|
| electron-mcp | MCP サーバー | **不可** | スクリーンキャプチャ/UI操作機能なし |
| osascript (System Events) | キー送信・ウィンドウ操作 | **可** | wgpu 描画内容は読めない |
| AXUIElement (Swift) | ウィンドウ属性取得 | **可** | タイトル・位置・サイズ・フォーカス |
| screencapture -R | 座標指定キャプチャ | **可** | Screen Recording 権限 + 再起動が必要 |
| ScreenCaptureKit | ウィンドウ単位キャプチャ | **可** | macOS 15+ 公式API。権限 + 再起動が必要 |
| XCTest UI | UI テストフレームワーク | **不可** | Xcode フル版が必要、Cargo 統合困難 |
| cliclick | マウス・キーボード操作 | **将来** | ドラッグ&ドロップ（Phase 4）で有用 |

### 重要な制約
- wgpu でレンダリングされた内容は Accessibility API からテキストとして読めない
- 描画内容の検証はスクリーンショット + 画像比較（またはAI視覚分析）が必要
- Screen Recording 権限は付与後に OS 再起動が必要

### 権限モデル
Swift/osascript のユーティリティスクリプトをコンパイル済みバイナリとして配置し、
そのバイナリに Screen Recording 権限を付与する。
Claude Code からはユーティリティスクリプトを呼ぶだけで権限問題を回避できる。

```
tools/test-utils/
├── window-info       # Swift: AXUIElement でウィンドウ属性を取得
├── capture-window    # Swift: ScreenCaptureKit でウィンドウキャプチャ
├── send-keys         # osascript: キーストローク送信
└── README.md         # 権限設定手順
```

## タスク

### Layer 1: ヘッドレステスト（GUI 不要・全 CI で実行可能）

- [ ] `--headless` モードを `main.rs` に追加
  - PTY spawn → `echo SDIT_HEADLESS_OK` → Grid 確認 → exit(0)
  - タイムアウト（5秒）で exit(1)
  - winit/wgpu を一切初期化しない
- [ ] `crates/sdit/tests/smoke_headless.rs` を作成
  - `CARGO_BIN_EXE_sdit` でバイナリパス取得
  - `--headless` で起動、exit code 0 を確認
  - stderr にパニック・エラーがないことを確認
- [ ] macOS 26 PTY ioctl 互換性の退行テスト
  - `Pty::spawn()` → `resize()` の順序が正しいことを検証

### Layer 2: GUI スモークテスト（ディスプレイ環境で実行）

- [ ] `SDIT_SMOKE_TEST=1` モードを `main.rs` に追加
  - PTY 起動 + 1フレーム描画後に `event_loop.exit()`
  - exit code 0 で正常終了
- [ ] `crates/sdit/tests/smoke_gui.rs` を作成（`#[ignore]`）
  - `CARGO_BIN_EXE_sdit` + `SDIT_SMOKE_TEST=1` で起動
  - 15秒タイムアウト、exit code 0 を確認

### Layer 3: GUI 操作テスト（ユーティリティスクリプト経由）

- [ ] `tools/test-utils/` にユーティリティスクリプトを作成
  - `window-info`: Swift — AXUIElement でウィンドウ属性（タイトル、位置、サイズ、フォーカス）を JSON 出力
  - `capture-window`: Swift — ScreenCaptureKit でウィンドウを PNG キャプチャ
  - `send-keys`: osascript — 指定プロセスにキーストローク送信
- [ ] 権限設定手順を README.md に記載
  - Screen Recording 権限の付与方法
  - 再起動が必要な旨の注意書き
  - `tccutil` によるリセット手順
- [ ] `crates/sdit/tests/gui_interaction.rs` を作成（`#[ignore]`）
  - バイナリ起動 → window-info でウィンドウ存在確認
  - send-keys でキー入力 → capture-window でスクリーンショット
  - スクリーンショットが空でないことを確認（画像サイズ > 閾値）

### 共通

- [ ] Lint・ビルド確認
- [ ] セキュリティレビュー
- [ ] knowhow 記録（macOS 26 PTY 互換性、GUI テスト権限モデル）

## 実装優先度

1. **Layer 1**（最優先）: 全環境で動く。macOS 26 PTY 問題の退行防止
2. **Layer 2**（次点）: バイナリ起動確認。実装コスト低い
3. **Layer 3**（Screen Recording 権限付与後）: 描画内容の視覚的検証

## 対象クレート
- `crates/sdit/` (バイナリ + 統合テスト)
- `crates/sdit-core/` (PTY テスト強化)
- `tools/test-utils/` (新規: GUI テストユーティリティ)

## 参照
- `crates/sdit-core/tests/headless_pipeline.rs` (既存統合テストパターン)
- `docs/knowhow/pty-threading-model.md` (PTY スレッドモデル)
