# capture-window の Screen Recording 権限

## 概要

`capture-window`（ScreenCaptureKit ベース）が `CGS_REQUIRE_INIT` アサーションで落ちる問題とその対策。

## 症状

```
Assertion failed: (did_initialize), function CGS_REQUIRE_INIT, file CGInitialization.c, line 44.
```

exit code 134（SIGABRT）で異常終了する。
現在は SIGABRT トラップにより診断情報が stderr に出力される。

## 原因の切り分け

`CGS_REQUIRE_INIT` は2つの原因で発生する:

### 1. Screen Recording 権限が未付与

```bash
# 確認方法
./tools/test-utils/capture-window --request-access
# → "権限が拒否されました" なら権限の問題
```

### 2. window server セッションの初期化失敗（権限は付与済み）

`CGPreflightScreenCaptureAccess()` が `true` を返すにもかかわらず、`SCScreenshotManager.captureImage` の段階でクラッシュするケース。

- **`SCShareableContent`（ウィンドウ一覧取得）は動作する**が、実際のピクセルキャプチャは window server とのフルセッションが必要
- Claude Code（VSCode 拡張）のプロセスツリーから起動した子プロセスが、window server セッションを正しく継承していない場合に発生する
- `screencapture` コマンドはシステムバイナリで特殊なエンタイトルメント（`com.apple.security.temporary-exception.mach-lookup.global-name`等）を持つため、この問題の影響を受けない

## 対策

### 自動フォールバック（推奨）

`capture-window` に `screencapture -R` フォールバックが組み込み済み。SIGABRT トラップ内で AXUIElement からウィンドウ座標を取得し、`screencapture -R` で代替キャプチャする。呼び出し側の変更は不要。

```
SCScreenshotManager.captureImage → SIGABRT (CGS_REQUIRE_INIT)
  → SIGABRT ハンドラ発動
  → AXUIElement でウィンドウ座標取得
  → screencapture -R でキャプチャ
  → exit 0
```

stderr に `Info: SCScreenshotManager failed (CGS_REQUIRE_INIT), trying screencapture fallback...` が出力されるが、exit code は 0。

### 手動で screencapture を使う場合

```bash
# window-info (JSON) → python で座標抽出 → screencapture
INFO=$(./tools/test-utils/window-info --pid $PID)
X=$(echo "$INFO" | python3 -c "import sys,json; d=json.load(sys.stdin); print(int(d['position']['x']))")
Y=$(echo "$INFO" | python3 -c "import sys,json; d=json.load(sys.stdin); print(int(d['position']['y']))")
W=$(echo "$INFO" | python3 -c "import sys,json; d=json.load(sys.stdin); print(int(d['size']['width']))")
H=$(echo "$INFO" | python3 -c "import sys,json; d=json.load(sys.stdin); print(int(d['size']['height']))")
screencapture -R"${X},${Y},${W},${H}" output.png
```

### VSCode の再起動

権限変更後は VSCode を完全に終了して再起動する。これで window server セッションが正しく初期化される場合がある。

1. システム環境設定 > プライバシーとセキュリティ > 画面収録 で権限を確認
2. VSCode（または使用中のターミナル）に権限が付与されていることを確認
3. VSCode を完全に終了して再起動する

### 権限の確認・要求

```bash
# 権限ステータスチェック
./tools/test-utils/capture-window --request-access

# 総合チェック（capture-window + プロセス情報）
./tools/test-utils/check-screen-recording.sh

# 設定画面を開く
./tools/test-utils/check-screen-recording.sh --request
```

## 注意点

- `screencapture` はウィンドウの影やタイトルバーを含む場合がある。ピクセル単位の比較テストには向かない
- `build.sh` でリビルドするとバイナリハッシュが変わり、macOS が権限をリセットする場合がある
- CI 環境（ヘッドレス）では Screen Recording 権限自体が使えない場合がある

## ディスプレイスリープ中の問題

**症状**: `screencapture -x` が黒一色の画像（ファイルサイズ約 90KB）を返す。`window-info` でウィンドウサイズが `(0, 0)` になる。

**原因**: `system_profiler SPDisplaysDataType` で `Display Asleep: Yes` の場合、ディスプレイが省電力スリープ中。screencapture は真っ黒な画像を返し、AXUIElement のウィンドウサイズ取得も 0 になる。

**確認方法**:
```bash
system_profiler SPDisplaysDataType | grep "Display Asleep"
# → "Display Asleep: Yes" なら問題あり
```

**対処法**: ディスプレイをウェイクアップするか、GUI テストを実施できる状態でアクセスする（マウス移動、キー入力等）。ヘッドレス環境（Claude Code からの自動実行等）では GUI テストの実施自体が不可能なため、`cargo test` のユニットテストで代替検証を行う。

**スクリーンショットの判別**: ファイルサイズが常に同じ（例: 90,901 bytes）場合、すべてのキャプチャが同一の黒画像である可能性が高い。
