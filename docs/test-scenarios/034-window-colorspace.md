# 034: ウィンドウ色空間設定（sRGB / Display P3）確認

## 目的
`[window] colorspace` 設定により wgpu surface のピクセルフォーマットが切り替わることを確認する。

## 前提条件
- `cargo build --package sdit`
- macOS（Display P3 対応ディスプレイ推奨）

## 手順

### A: デフォルト（sRGB）

1. 設定ファイルに `colorspace` を指定しない（またはコメントアウト）
2. SDIT を `RUST_LOG=info` で起動する
3. ウィンドウが正常に表示されることを確認する
4. ログに sRGB 関連のフォーマット情報が出力されていることを確認する（任意）
5. SDIT を終了する

### B: colorspace = "srgb"（明示指定）

1. 設定ファイルに `[window]` セクションで `colorspace = "srgb"` を追加する
2. SDIT を起動し、正常に表示されることを確認する
3. SDIT を終了する

### C: colorspace = "display-p3"

1. 設定ファイルに `colorspace = "display-p3"` を設定する
2. SDIT を `RUST_LOG=info` で起動する
3. ウィンドウが正常に表示されることを確認する
4. sRGB 時と比較して色味が異なる可能性がある（P3 対応ディスプレイの場合）
5. SDIT を終了する

### D: 不正値

1. 設定ファイルに `colorspace = "invalid"` を設定する
2. SDIT を起動し、デフォルト（sRGB）にフォールバックして正常表示されることを確認する

## 期待結果
- デフォルトおよび `"srgb"` 指定時は sRGB フォーマットで描画される
- `"display-p3"` 指定時は `Bgra8UnormSrgb` フォーマットが選択される
- 不正値はデフォルトにフォールバックし、クラッシュしない
- いずれの設定でもテキスト描画・カラー表示が正常に動作する
