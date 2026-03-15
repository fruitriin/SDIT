# wgpu サーフェスフォーマットと sRGB 二重ガンマ補正

## 発見日
2026-03-15

## 問題
ターミナル全体の色が薄くグレーがかって見える。True color テストは通過しており、ANSI パレットの色値自体は正しい。

## 根本原因
wgpu のサーフェスフォーマットに `Bgra8UnormSrgb` を使用すると、GPU がフラグメントシェーダの出力を **線形色空間の値として解釈し、sRGB エンコーディングを自動適用** する。

しかし SDIT のシェーダに渡す色値は `hex_to_rgba(0xRR, 0xGG, 0xBB)` で、sRGB 空間の値を `/ 255.0` で線形マッピングしただけ。これは **既に sRGB 空間の値** なので、GPU がさらに sRGB エンコーディングを適用すると **ガンマ補正が二重** にかかり、全体的に薄くなる。

## 解決策
サーフェスフォーマットを `Bgra8Unorm`（非 sRGB）に変更する。これにより GPU はシェーダ出力をそのまま書き込み、sRGB 値がそのまま表示される。

```rust
// 非 sRGB フォーマットを優先して選択
let non_srgb = caps.formats.iter().copied().find(|f| !f.is_srgb());
```

## 正しい sRGB パイプライン（参考）
本来正しい方法は `*Srgb` フォーマットを使いつつ、シェーダに渡す前に色を sRGB → 線形変換すること:
```
CPU: sRGB hex → sRGB float → linear float（pow 2.2）
Shader: linear float で演算
GPU: linear → sRGB 自動変換（*Srgb フォーマット）
```
ただしターミナルエミュレータでは色の演算がほぼないため、非 sRGB フォーマットで十分。

## 判別方法
- 色が全体的に「白っぽい」「薄い」「グレーがかっている」場合はガンマ二重適用を疑う
- `wgpu::TextureFormat::*Srgb` を使っているか確認
