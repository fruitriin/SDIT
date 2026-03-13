# tools/test-utils — AI エージェント向けガイド

## 概要

macOS GUI テスト用のユーティリティスクリプト群。
SDIT のウィンドウ操作・画面キャプチャ・キー入力送信を自動化する。

## アーキテクチャ

```
tools/test-utils/
├── window-info.swift      # AXUIElement → JSON（ウィンドウ属性取得）
├── capture-window.swift   # ScreenCaptureKit → PNG（スクリーンショット）
├── render-text.swift      # CoreText → PNG（対照群テキストレンダリング）
├── verify-text.swift      # OCR + 輝度 + SSIM 一括検証（トークン効率最大化）
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

### CJK 対照群テストパターン

```bash
# 1. 対照群画像を生成（CoreText で正解レンダリング）
./tools/test-utils/render-text --mono --cell-info "こんにちは世界" tmp/reference-cjk.png

# 2. SDIT で同じテキストを表示してキャプチャ
./tools/test-utils/capture-window sdit tmp/sdit-cjk.png

# 3. 視覚比較（AI エージェントが両画像を読んで差分を判定）
```

render-text オプション:
- `--mono`: 等幅グリッド描画（ターミナル風、CJK比較に推奨）
- `--cell-info`: セル境界座標を JSON で出力（右端クリッピング検出に有用）
- `--font <name>`: フォント名（SDIT の設定と揃える）
- `--size <pt>`: フォントサイズ
- `--bg/--fg <hex>`: 背景色/テキスト色

### テキスト検証パターン（トークン最適化）

```bash
# 1. 対照群生成 + セル境界 JSON 保存
./tools/test-utils/render-text --mono --cell-info "テスト文字列" tmp/ref.png | tail -n +2 > tmp/cells.json

# 2. SDIT キャプチャ
./tools/test-utils/capture-window sdit tmp/sdit.png

# 3. 一括検証（エージェントはテキスト出力のみ読む → 画像トークン不要）
./tools/test-utils/verify-text tmp/sdit.png "テスト文字列" \
    --cells tmp/cells.json \
    --reference tmp/ref.png
```

verify-text は 3 種類のチェックを一括実行し、構造化テキストレポートを返す:
- **OCR 照合**: Vision.framework で認識、期待テキストと比較
- **輝度分析**: セル内インク有無 + 右端クリッピング検出
- **SSIM 比較**: 対照群とのセル単位構造類似度

exit code: 0=全PASS, 1=エラー, 3=いずれかFAIL

### Exit code 規約

| ツール | 0 | 1 | 2 |
|---|---|---|---|
| window-info | 成功 | ウィンドウ未発見 | — |
| capture-window | 成功 | エラー | 権限なし |
| send-keys.sh | 成功 | 引数不正 | プロセス未発見/権限なし |

## コード変更時の注意

### セキュリティ
- **send-keys.sh**: AppleScript に変数を埋め込む際は必ずエスケープする（H-1 対応済み）
- **send-keys.sh**: `pgrep -x` で PID を取得し、AppleScript で `unix id` ベースのプロセス指定（M-3 対応済み）
- **capture-window**: 出力パスをワーキングディレクトリ配下に制限（パストラバーサル防止、M-1 対応済み）
- **capture-window**: `--pid <pid>` オプションで PID 直接指定が可能。プロセス名検索はフルパス優先+basename フォールバック（警告付き）（M-3 対応済み）

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
