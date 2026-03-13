# 統合テストパターン（Phase 2.5 知見）

## 3層テスト構成

| Layer | 特徴 | 環境依存 | テストファイル |
|---|---|---|---|
| Layer 1: ヘッドレス | PTY→VTE→Grid パイプラインのみ | なし（CI OK） | `smoke_headless.rs` |
| Layer 2: GUI スモーク | winit+wgpu で1フレーム描画 | ディスプレイ必要 | `smoke_gui.rs` |
| Layer 3: GUI 操作 | ウィンドウ操作+スクリーンショット | ディスプレイ+権限必要 | `gui_interaction.rs` |

## `--headless` モードのパターン

- `main()` 冒頭で `std::env::args()` をチェック、GUI 初期化を完全スキップ
- PTY を spawn → 出力を Terminal に流す → Grid を検査 → exit code で結果通知
- タイムアウト（5秒）で exit(1)
- EOF / EIO 後も Grid を最終確認してから判定する（PTY はデータ全部読む前に EOF/EIO になりうる）

## `SDIT_SMOKE_TEST=1` のパターン

- `cfg!(debug_assertions)` でガードし、release ビルドでは無効化
- `RedrawRequested` ハンドラ内で `event_loop.exit()` を呼ぶ
- 1フレーム描画完了 = GPU パイプラインが正常動作したことの検証

## テストでの子プロセス待機パターン

```rust
fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Option<ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}
```

`Command::timeout()` は nightly 限定のため、`spawn + try_wait` ポーリングを使用。

## macOS GUI テストユーティリティ

- `window-info`: AXUIElement (Accessibility API) でウィンドウ属性取得
- `capture-window`: ScreenCaptureKit (macOS 15+) でウィンドウ単位キャプチャ
- `render-text`: CoreText で対照群テキスト画像を生成（`--mono` でターミナル風等幅）
- `verify-text`: OCR + 輝度分析 + SSIM の3層一括検証（テキストレポートのみ返す）
- `send-keys.sh`: osascript の System Events でキーストローク送信
  - **重要**: AppleScript に変数を埋め込む際はバックスラッシュと二重引用符のエスケープが必須

## テキスト描画の自動検証パターン

**目的**: エージェントが画像を読まずにテキストレポートだけで描画品質を判定する（トークン節約）

### パターン A: ASCII テキスト存在確認（最小コスト）
```bash
verify-text tmp/capture.png "EXPECTED_TEXT"
# OCR のみ。--cells/--reference 不要
```
用途: 001-basic-echo, 014-font-size-change

### パターン B: CJK 全角文字の品質検証（3層フル）
```bash
render-text --mono --cell-info "テスト文字列" tmp/ref.png | tail -n +2 > tmp/cells.json
verify-text tmp/capture.png "テスト文字列" --cells tmp/cells.json --reference tmp/ref.png
```
用途: 009-cjk-display

### パターン C: 特殊文字（絵文字・リガチャ）の品質検証
```bash
render-text --mono --cell-info "🎉 -> =>" tmp/ref.png | tail -n +2 > tmp/cells.json
verify-text tmp/capture.png "🎉 -> =>" --cells tmp/cells.json --reference tmp/ref.png
```
用途: 020-color-emoji, 023-opentype-ligature
注意: OCR は絵文字/リガチャの認識精度が低い場合がある → SSIM スコアで補完判定

### exit code の解釈
- `0`: 全チェック PASS
- `1`: ツールエラー（引数不正等）
- `3`: いずれかのチェック FAIL → レポートの `[RESULT]` 行で詳細を確認

## macOS 権限モデル

- Screen Recording 権限は付与後に OS 再起動が必要
- Accessibility 権限はターミナルアプリに付与する
- Swift ユーティリティはコンパイル済みバイナリに権限を付与する
