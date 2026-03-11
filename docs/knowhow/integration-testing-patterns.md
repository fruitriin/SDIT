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
- `send-keys.sh`: osascript の System Events でキーストローク送信
  - **重要**: AppleScript に変数を埋め込む際はバックスラッシュと二重引用符のエスケープが必須

## macOS 権限モデル

- Screen Recording 権限は付与後に OS 再起動が必要
- Accessibility 権限はターミナルアプリに付与する
- Swift ユーティリティはコンパイル済みバイナリに権限を付与する
