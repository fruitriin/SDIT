# IME 入力サポート実装ノウハウ

## 概要

winit の `WindowEvent::Ime` を使って macOS IME（日本語入力）を実装したときの知見。

## winit IME API

### 有効化

```rust
window.set_ime_allowed(true);
```

ウィンドウ作成後すぐに呼ぶ。`create_window()` で `Ok(w)` を得た直後、`Arc::new(w)` の前に呼ぶのが自然。

### イベント

```rust
WindowEvent::Ime(ime_event) => match ime_event {
    winit::event::Ime::Enabled => { /* IME 有効化 */ }
    winit::event::Ime::Preedit(text, cursor) => {
        // 変換中テキスト。text が空 = プリエディット終了
        // cursor: Option<(usize, usize)> = バイトオフセット範囲
    }
    winit::event::Ime::Commit(text) => {
        // 確定テキスト → PTY へ送信
    }
    winit::event::Ime::Disabled => { /* IME 無効化 */ }
}
```

### カーソル位置通知

```rust
window.set_ime_cursor_area(
    PhysicalPosition::new(x, y),   // f64
    PhysicalSize::new(w, h),       // f64
);
```

OS に IME 変換ウィンドウの表示位置を伝える。`redraw_session` の末尾（`request_redraw` の前）で毎フレーム呼ぶ。プリエディット中はプリエディット末尾の位置を通知する。

## ブラケットペーストと IME Commit の関係

IME の確定テキストが複数文字（`text.len() > 1`）で、かつ端末が BRACKETED_PASTE モードの場合、ブラケットシーケンスで包んで送信する。

ただし、テキスト内にブラケットシーケンスが混入していないかサニタイズが必要（Terminal Injection 攻撃防止）:

```rust
let sanitized = text.replace("\x1b[200~", "").replace("\x1b[201~", "");
let mut v = b"\x1b[200~".to_vec();
v.extend_from_slice(sanitized.as_bytes());
v.extend_from_slice(b"\x1b[201~");
```

1文字の場合はブラケット不要（通常の文字入力と同じ）。

## プリエディット描画

### 方針

1. `update_from_grid()` で通常グリッドを描画
2. `CellPipeline::overwrite_cell()` でプリエディット文字を上書き

### overwrite_cell の実装

`wgpu::Queue::write_buffer` はバイトオフセット指定で部分書き込みができる:

```rust
pub fn overwrite_cell(&self, queue: &wgpu::Queue, index: usize, vertex: &CellVertex) {
    if index as u32 >= self.cell_count {
        return; // 範囲外書き込み防止
    }
    let byte_offset = (index * std::mem::size_of::<CellVertex>()) as wgpu::BufferAddress;
    queue.write_buffer(&self.vertex_buffer, byte_offset, bytemuck::bytes_of(vertex));
}
```

### プリエディット背景色

通常背景より少し明るくすることで視覚的に区別できる:

```rust
let bg = colors.background;
let preedit_bg = [
    (bg[0] + 0.15).min(1.0),
    (bg[1] + 0.15).min(1.0),
    (bg[2] + 0.15).min(1.0),
    1.0,
];
```

## 全角文字幅の判定

`Cell::is_wide_char()` は存在しない。`CellFlags::WIDE_CHAR` はグリッドセル用なので、プリエディット（グリッドに載っていない文字）には使えない。

簡易実装として主要な CJK Unicode ブロックをカバーする関数を実装:

```rust
fn char_cell_width(c: char) -> usize {
    let cp = c as u32;
    matches!(cp,
        0x3041..=0x33FF  // ひらがな、カタカナ、CJK
        | 0x4E00..=0x9FFF  // CJK 統合漢字
        // ...他のブロック
    ).then_some(2).unwrap_or(1)
}
```

完全対応には `unicode-width` クレートが必要だが、依存を避ける場合は主要ブロックのカバーで十分。

## CellVertex の UV 形式

`uv: [f32; 4]` は `[u_min, v_min, u_max, v_max]` であり、`update_from_grid` と同じ計算式を使う:

```rust
let uv = [
    r.x as f32 / atlas_size,
    r.y as f32 / atlas_size,
    (r.x + r.width) as f32 / atlas_size,
    (r.y + r.height) as f32 / atlas_size,
];
```

## アトラスのアップロードタイミング

プリエディット描画でアトラスに新しいグリフが追加された場合、`upload_if_dirty` を呼び直す必要がある。`update_from_grid` 内でも呼ばれているが、プリエディット描画後に再度呼ぶ。

## 既知の制限

- `PreeditState.cursor_offset` は将来のプリエディットカーソル下線描画用に保持しているが現時点では未使用
- `char_cell_width` は主要 CJK ブロックのみカバー（`unicode-width` クレートで完全対応可能）
