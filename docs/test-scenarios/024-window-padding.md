# 024: ウィンドウパディング確認

## 目的
`window.padding_x` / `window.padding_y` を設定したとき、テキストがウィンドウ端から指定ピクセル分だけ内側に描画されることを確認する。

## 前提条件
- `cargo build --package sdit`
- `tools/test-utils/build.sh`
- Screen Recording 権限 + OS 再起動
- Accessibility 権限

## 手順

### 24-A: パディングなし（デフォルト）

1. SDIT をデフォルト設定（padding_x=0, padding_y=0）でバックグラウンド起動する
2. window-info でウィンドウの存在を確認する（最大 15 秒ポーリング）
3. send-keys で `echo PADDING_TEST_ZERO` を入力・Return を送信する
4. 2 秒待機する
5. capture-window でスクリーンショットを撮る（`tmp/024-padding-zero.png`）
6. verify-text で "PADDING_TEST_ZERO" の表示確認（OCR 照合）
7. SDIT を終了する

### 24-B: パディングあり（padding_x=16, padding_y=8）

1. 設定ファイルを作成する:
   ```toml
   [window]
   padding_x = 16
   padding_y = 8
   ```
2. SDIT を当該設定でバックグラウンド起動する
3. window-info でウィンドウの存在を確認する
4. send-keys で `echo PADDING_TEST_SET` を入力・Return を送信する
5. 2 秒待機する
6. capture-window でスクリーンショットを撮る（`tmp/024-padding-set.png`）
7. verify-text で "PADDING_TEST_SET" の表示確認（OCR 照合）
8. SDIT を終了する

### 24-C: 画像比較（パディング余白の目視確認）

1. 24-A と 24-B のスクリーンショットをエージェントが読み込む
2. テキスト開始位置を比較する:
   - 24-A: テキストがウィンドウ左端・上端に密着または近い
   - 24-B: テキストが左端から約 16px、上端から約 8px 内側に開始している
3. 差分が視覚的に確認できれば PASS

### 24-D: Hot Reload（パディング変更の動的反映）

1. padding_x=0, padding_y=0 で SDIT を起動する
2. SDIT が起動中に設定ファイルを padding_x=20, padding_y=10 に変更する
3. 3 秒待機する（hot reload 検知を待つ）
4. capture-window でスクリーンショットを撮る（`tmp/024-padding-hotreload.png`）
5. エージェントが画像を読み込み、テキストがウィンドウ端から内側にオフセットされていることを確認する
6. SDIT を終了する

## 自動検証

```bash
# 24-A
./tools/test-utils/verify-text tmp/024-padding-zero.png "PADDING_TEST_ZERO"

# 24-B
./tools/test-utils/verify-text tmp/024-padding-set.png "PADDING_TEST_SET"
```

## 期待結果

- **24-A**: `verify-text` が exit 0 / OCR で "PADDING_TEST_ZERO" が認識される
- **24-B**: `verify-text` が exit 0 / OCR で "PADDING_TEST_SET" が認識される
- **24-C**: 24-B のスクリーンショットで、24-A に比べてテキストが内側にオフセットされている
- **24-D**: Hot Reload 後のスクリーンショットで、パディングが反映されてテキストが内側にある

## クランプ動作確認（ユニットテスト）

`cargo test -p sdit-core window_padding_clamp` でパディングクランプのテストを実行できる。
padding_x=500 → clamped_padding_x()=200、padding_y=300 → clamped_padding_y()=200 であることを確認済み。

## クリーンアップ
- SDIT プロセスを終了する
- `tmp/024-*.png` を削除する
- 一時設定ファイルを削除する

## 関連
- `crates/sdit-core/src/config/mod.rs` — WindowConfig.padding_x/padding_y
- `crates/sdit/src/render.rs` — パディングオフセット適用
- `crates/sdit/src/input.rs` — マウス座標補正
- Phase 14.3 計画: `docs/plans/phase14.3-window-padding.md`
