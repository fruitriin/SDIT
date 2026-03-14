# Phase 25.3: ウィンドウ色空間設定（macOS Display P3）

## 要望

macOS の wide color display（Display P3）対応として、ウィンドウの色空間を
sRGB または Display P3 から選択できるようにする。

Display P3 では sRGB より約 25% 広い色域が使えるため、
鮮やかなターミナルカラーが得られる。

Ghostty: `window-colorspace = "srgb" | "display-p3"`

## 現状

wgpu + winit の現在の実装は sRGB のみ。macOS の Metal/wgpu では
surface のピクセルフォーマットを `Bgra8Unorm` (sRGB) または
`Bgra8UnormSrgb` + P3 変換で切り替えることが可能。

## 実装方針

1. `[window] colorspace = "srgb" | "display-p3"` 設定を追加（デフォルト: `"srgb"`）
2. `GpuContext` の surface 設定で `wgpu::TextureFormat` を切り替える
3. macOS のみ有効（`#[cfg(target_os = "macos")]`）

### 注意点
- Display P3 は macOS 10.15 Catalina 以降のみサポート
- レンダーパイプラインのシェーダーも色変換を考慮する必要がある場合がある

## 変更対象

- `crates/sdit-core/src/config/mod.rs` — `[window] colorspace` 追加（macOS のみ）
- `crates/sdit-core/src/render/pipeline.rs` — surface フォーマット切り替え

## 実装結果（2026-03-15 完了）

- `WindowColorspace` enum（Srgb/DisplayP3）と `[window] colorspace` 設定を追加
- `GpuContext::new(prefer_wide_color: bool)` パラメータを追加
- `display-p3` 時に `Bgra8UnormSrgb` フォーマットを優先（利用不可時はフォールバック＋ログ）
- Critical 修正: `matches!()` マクロ使用 + フォールバック時のデバッグログ追加

テスト: 444 件 PASS

## セキュリティ影響

なし（Critical/High 修正済み）

## 参照

- Ghostty: `refs/ghostty/src/config/Config.zig` `window-colorspace`
- wgpu: `TextureFormat::Bgra8Unorm` vs `TextureFormat::Bgra8UnormSrgb`
