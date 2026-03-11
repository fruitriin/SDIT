# tools/test-utils — GUI テストユーティリティ

macOS 向けの GUI テスト補助ツール群。
SDIT の `crates/sdit/tests/gui_interaction.rs` から呼び出される。

## ツール一覧

| ツール | 言語 | 用途 |
|---|---|---|
| `window-info` | Swift | AXUIElement でウィンドウ属性を JSON 出力 |
| `capture-window` | Swift | ScreenCaptureKit でウィンドウを PNG キャプチャ |
| `send-keys.sh` | bash | osascript でキーストロークを送信 |
| `build.sh` | bash | Swift スクリプトをコンパイル |

---

## ビルド手順

```bash
cd tools/test-utils
./build.sh
```

`window-info` と `capture-window` のバイナリが生成される。

### 必要な環境

- macOS 15 (Sequoia) 以降（ScreenCaptureKit の `SCScreenshotManager` API）
- Xcode Command Line Tools（`swiftc` が使えること）

```bash
xcode-select --install
```

---

## 権限設定

### Screen Recording（`capture-window` に必要）

1. **System Settings → Privacy & Security → Screen Recording** を開く
2. リストに `capture-window` が表示されていなければ、`+` ボタンで追加する
3. トグルをオンにする
4. **OS を再起動する**（再起動しないと権限が有効にならない）

> **注意**: 権限付与後は必ず再起動してください。再起動なしでは `SCStreamError` が発生します。

### 権限のリセット（テスト環境のクリーンアップ用）

```bash
# capture-window の Screen Recording 権限をリセット
tccutil reset ScreenCapture

# その後、再度 System Settings から権限を付与 → 再起動
```

### Accessibility（`send-keys.sh` に必要）

1. **System Settings → Privacy & Security → Accessibility** を開く
2. `send-keys.sh` を実行するターミナルアプリを許可する
   （例: Terminal.app、iTerm2、Ghostty など）

---

## 各ツールの使い方

### window-info

```bash
# ビルド後のバイナリを使用
./window-info sdit

# 出力例:
# {
#   "focused" : true,
#   "pid" : 12345,
#   "position" : { "x" : 100, "y" : 200 },
#   "size" : { "height" : 600, "width" : 800 },
#   "title" : "sdit"
# }
```

- ウィンドウが見つからない場合は `exit 1`
- 権限エラーはなし（AXUIElement は Accessibility 不要で読み取り可能）

### capture-window

```bash
# ビルド後のバイナリを使用
# 出力先はプロジェクトルートの tmp/ を使う
./capture-window sdit ../../tmp/test-capture.png

# 成功時の出力:
# Captured: ../../tmp/test-capture.png (1600x1200px)
```

- Screen Recording 権限がない場合は `exit 2`
- ウィンドウが見つからない場合は `exit 1`
- 出力先ディレクトリが存在しない場合は自動作成

### send-keys.sh

```bash
./send-keys.sh sdit "hello world"
# → sdit プロセスにキーストロークを送信

./send-keys.sh sdit "echo test"
# → Enter は含まれない（必要なら \n を追加して改変）
```

- プロセスが見つからない場合は `exit 2`
- Accessibility 権限がない場合は `exit 2`

---

## GUI テストの実行

```bash
# プロジェクトルートから
cargo test --test gui_interaction -- --ignored

# 特定のテストのみ
cargo test --test gui_interaction window_appears -- --ignored
```

`#[ignore]` 属性付きテストのため、`-- --ignored` フラグが必要。

---

## トラブルシューティング

| 症状 | 原因 | 対処 |
|---|---|---|
| `SCStreamError` / exit 2 | Screen Recording 権限なし | 権限付与 → 再起動 |
| `osascript` が失敗 | Accessibility 権限なし | ターミナルを Accessibility に追加 |
| バイナリが見つからない | ビルド未実施 | `./build.sh` を実行 |
| ウィンドウが見つからない | sdit が起動していない | sdit を先に起動する |
