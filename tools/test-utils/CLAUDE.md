# tools/test-utils — AI エージェント向けガイド

## 概要

macOS GUI テスト用のユーティリティスクリプト群。
SDIT のウィンドウ操作・画面キャプチャ・キー入力送信を自動化する。

## アーキテクチャ

```
tools/test-utils/
├── window-info.swift      # AXUIElement → JSON（ウィンドウ属性取得）
├── capture-window.swift   # ScreenCaptureKit → PNG（スクリーンショット）
├── send-keys.sh           # osascript System Events（キーストローク送信）
├── build.sh               # swiftc でコンパイル
├── README.md              # 人間向けドキュメント
└── CLAUDE.md              # このファイル
```

## 使い方（エージェントがテストを実行する場合）

### 前提条件
1. `./build.sh` でコンパイル済みであること
2. Screen Recording 権限 + OS 再起動 済みであること（capture-window 用）
3. Accessibility 権限 が付与されていること（send-keys.sh 用）

### テスト実行パターン

```bash
# 1. SDIT を起動（バックグラウンド）
cargo run --package sdit &
SDIT_PID=$!
sleep 2  # ウィンドウ描画を待つ

# 2. ウィンドウ存在確認
./tools/test-utils/window-info sdit

# 3. キー入力送信
./tools/test-utils/send-keys.sh sdit "echo hello"

# 4. スクリーンショット
./tools/test-utils/capture-window sdit tmp/test-capture.png

# 5. クリーンアップ
kill $SDIT_PID
```

### Exit code 規約

| ツール | 0 | 1 | 2 |
|---|---|---|---|
| window-info | 成功 | ウィンドウ未発見 | — |
| capture-window | 成功 | エラー | 権限なし |
| send-keys.sh | 成功 | 引数不正 | プロセス未発見/権限なし |

## コード変更時の注意

### セキュリティ
- **send-keys.sh**: AppleScript に変数を埋め込む際は必ずエスケープする（H-1 対応済み）
- **capture-window**: 出力パスのバリデーションは未実装（M-1 記録済み）
- **window-info / capture-window**: プロセス名のフルパス比較は未実装（M-3 記録済み）

### 制約
- Swift コードは `swiftc` でコンパイルが必要（インタプリタ実行不可、ScreenCaptureKit リンクが必要）
- ScreenCaptureKit は macOS 15+ のみ
- Screen Recording 権限はコンパイル済みバイナリに付与する（ソースでなく）
- 権限付与後の OS 再起動は省略不可

### 一時ファイル
- スクリーンショット等の出力はプロジェクトルートの `tmp/` に書き出す
- `/tmp/` は使用禁止（CLAUDE.local.md「一時ファイル方針」参照）

## テストシナリオの追加方法

1. `docs/test-scenarios/` にシナリオ .md を作成
2. `crates/sdit/tests/gui_interaction.rs` にテスト関数を追加（`#[ignore]` 付き）
3. テスト関数内で test-utils のツールを呼び出す
