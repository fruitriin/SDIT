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
  - **コマンド文字列のスペース**: `echo` や `print` 等のコマンドと引数の間のスペースは、1つの文字列引数に含めること。分割して送ると結合されてしまう（→ 下記「シェルコマンド入力のスペース問題」参照）
  - **英数キー先打ち必須**: `send-keys.sh` を呼ぶ前に毎回英数キー（key code 102）を送り IME モードを確定する（→ 下記「英数キー先打ちパターン」参照）

## 英数キー先打ちパターン

`send-keys.sh` でキーを送る前に**毎回**英数キー（key code 102）を先打ちして IME モードを確定する。
現在の IME 状態に依存しないため、テスト間の状態持ち越しを防ぐ。

```bash
# 英数キーを送る（IME を ASCII モードに確定）
osascript -e 'tell application "System Events" to key code 102'
sleep 0.1

# その後にコマンドを送る
./tools/test-utils/send-keys.sh sdit "echo hello world"
```

`send-keys.sh` を使うすべてのシナリオ手順で、keystroke の直前に英数キー先打ちを入れること。
詳細は `docs/knowhow/gui-test-ime-interference.md` を参照。

## シェルコマンド入力のスペース問題

`send-keys.sh` でシェルコマンドを入力する際、**コマンドと引数は1つの文字列として渡す**こと。

```bash
# ✅ 正しい: スペース込みで1文字列
./tools/test-utils/send-keys.sh sdit "echo こんにちは世界"

# ❌ 誤り: コマンドと引数を別々に送ると結合される
./tools/test-utils/send-keys.sh sdit "echo"
./tools/test-utils/send-keys.sh sdit "こんにちは世界"
# → "echoこんにちは世界" と入力され、コマンドとして認識されない
```

**代替手段: PTY 直接書き込み（最も安定）**

IME 干渉や AppleScript のタイミング問題を回避する方法として PTY デバイスへの直接書き込みがある:

```bash
# PTY デバイスを特定
ls /dev/ttys*  # SDIT が使用中の tty を確認

# 直接書き込み（\r で Enter 相当）
printf "echo こんにちは世界\r" > /dev/ttys002
```

PTY 直接書き込みは IME・AppleScript・権限の影響を受けないため、
CJK 文字や特殊文字を含むコマンドに特に有効。
ただし tty デバイス番号は起動のたびに変わるため `window-info` や `ps` で確認が必要。

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

## ディスプレイスリープ中の GUI テスト代替戦略

`Display Asleep: Yes` の環境では `capture-window` と `screencapture` の両方が失敗する（exit code 134）。
この状況では以下の代替戦略でテストを進める:

1. **ユニットテスト（`cargo test`）で設定・ロジックを検証**: パディングのクランプ、TOML デシリアライズ等は GUI 不要で確認できる
2. **ヘッドレステスト（`smoke_headless.rs`）でパイプライン検証**: PTY→VTE→Grid の動作確認
3. **シナリオを UNIT_ONLY として INDEX に登録**: GUI が必要な手順を文書化し、ディスプレイ起動時に実行できるよう残しておく

確認方法:
```bash
system_profiler SPDisplaysDataType | grep "Display Asleep"
# "Display Asleep: Yes" → GUI テスト不可、ユニットテストで代替
```

## 視覚的差分テスト（パディング等オフセット系機能）のパターン

パディング・マージン等「レイアウトオフセット」系の機能は OCR だけでは検証できない。
以下のアプローチを組み合わせる:

| 検証方法 | 内容 | 適用場面 |
|---|---|---|
| ユニットテスト | clamped_padding_x/y の値確認 | 設定値の正確性 |
| スクリーンショット比較 | パディング 0 vs パディングあり の2枚を目視比較 | テキスト開始位置のオフセット確認 |
| verify-text OCR | テキストが表示されているかの確認 | テキスト欠損がないことの確認 |

スクリーンショット比較では「テキストの開始 X 座標」を測定できれば機械的に確認できるが、
現状の verify-text ツールには座標比較機能がない。将来的な拡張候補として記録する。
