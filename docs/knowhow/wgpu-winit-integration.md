# wgpu 0.20 + winit 0.30 統合の知見

## ライフタイム問題: Surface と Window

wgpu の `Surface<'window>` は `Window` のライフタイムに縛られる。
`Arc<Window>` を使い `GpuContext<'static>` を返すパターンが安全:

```rust
pub fn new(window: &Arc<Window>) -> Result<GpuContext<'static>> {
    let surface = instance.create_surface(Arc::clone(window))?;
    // Arc が所有権を保持するため 'static は安全
}
```

`SditApp` が `Arc<Window>` と `GpuContext` を同じ構造体で保持する。

## ApplicationHandler パターン（winit 0.30）

- `resumed()` でウィンドウ + GPU コンテキストを初期化
- `window_event()` で各ウィンドウのイベントを処理
- `user_event()` で PTY からのカスタムイベントを受信
- `about_to_wait()` はイベント駆動描画では不要（連続描画には使う）

## Surface エラーハンドリング

```rust
Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
    gpu.resize(gpu.surface_config.width, gpu.surface_config.height);
}
```

Lost/Outdated は reconfigure で回復可能。

## GPU バッファの動的リサイズ

`ensure_capacity()` パターン:
- 現在の容量を追跡するフィールドを持つ
- 必要容量 > 現在容量のとき `saturating_mul(2)` で 2 倍に拡張
- 上限（64MB）を設けてオーバーフロー防御

## テクスチャアトラス

- シェルフアルゴリズムが最もシンプルで十分
- `R8Unorm` フォーマット（1byte/pixel のアルファマスク）
- `upload_if_dirty()` パターンで差分のみ GPU 転送
- release ビルドでも境界チェックを行う（`debug_assert` ではなく `if` ガード）

## cosmic-text でのグリフラスタライズ

- `Buffer` に1文字をセットしてシェーピング → `SwashCache::get_image_uncached()` でラスタライズ
- `monospace_em_width() * font_size` でセル幅を計算
- SwashContent::Mask → 1byte/pixel アルファデータ
