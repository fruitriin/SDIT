# GUI テストツールのプロセス識別

## 概要

テストユーティリティ（`window-info`, `send-keys.sh`）で同名プロセスが複数存在する場合、意図しないプロセスを操作してしまう問題。

## 症状

`window-info sdit` や `send-keys.sh sdit` は `pgrep -x` や `ps -eo pid,comm` で最初にマッチした PID を使う。開発中に複数の SDIT プロセスが起動していると、テスト対象ではないプロセスにキー入力やウィンドウ操作が送られる。

## ツール別の状況

| ツール | `--pid` オプション | 備考 |
|---|---|---|
| `capture-window` | あり | PID 指定でキャプチャ可能 |
| `window-info` | なし | プロセス名のみで検索 |
| `send-keys.sh` | なし | プロセス名のみで検索 |

## 回避策（当面）

テスト実行前に既存プロセスを全終了してからテスト対象を1つだけ起動する。

```bash
pkill -f "target/debug/sdit"
sleep 0.5
cargo run &
SDIT_PID=$!
```

## 恒久対策（TODO）

- `window-info` に `--pid` オプションを追加する
- `send-keys.sh` に `--pid` オプションを追加する

## テストシナリオでの注意

- PID を変数に保持し、クリーンアップで確実に `kill $PID` する
- テスト終了時に `trap` を設定して異常終了時もプロセスを回収する

```bash
SDIT_PID=""
cleanup() {
    [ -n "$SDIT_PID" ] && kill "$SDIT_PID" 2>/dev/null
}
trap cleanup EXIT
```
