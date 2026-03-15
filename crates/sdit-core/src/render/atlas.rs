//! テクスチャアトラス — グリフビットマップを1枚のテクスチャに詰め込む。
//!
//! シェルフアルゴリズムで矩形を配置する。同じ高さのグリフを横に並べてシェルフを構成し、
//! シェルフが満杯になったら新しいシェルフを作る。
//!
//! テクスチャフォーマットは `Rgba8Unorm`（RGBA 4bytes/pixel）。
//! カラー絵文字と通常グリフの両方を格納できる。

/// アトラス内のグリフ領域。UV 座標計算に使用する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// シェルフ1段の状態。
struct Shelf {
    /// シェルフの Y オフセット（ピクセル）。
    y: u32,
    /// シェルフの高さ（最初に配置したグリフの高さで決まる）。
    height: u32,
    /// 次の配置 X 位置。
    cursor_x: u32,
}

/// Atlas の一意識別子。グリフキャッシュが正しい Atlas のリージョンを参照しているか検証する。
pub type AtlasId = u64;

/// 次の Atlas ID を生成する。
fn next_atlas_id() -> AtlasId {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// `Rgba8Unorm` テクスチャアトラス。
///
/// CPU 側の `data` バッファに書き込み、`upload_if_dirty` で GPU テクスチャに転送する。
/// データは RGBA 4 bytes/pixel。通常グリフ（グレースケール昇格）とカラー絵文字の両方に対応。
pub struct Atlas {
    /// この Atlas の一意識別子。
    id: AtlasId,
    /// CPU ピクセルデータ（RGBA: 4 bytes/pixel）。
    data: Vec<u8>,
    /// テクスチャの一辺（正方形）。
    size: u32,
    /// シェルフのリスト。
    shelves: Vec<Shelf>,
    /// 次の新シェルフの Y 位置。
    next_shelf_y: u32,
    /// GPU テクスチャ。
    texture: wgpu::Texture,
    /// テクスチャビュー。
    texture_view: wgpu::TextureView,
    /// GPU へのアップロードが必要かどうか。
    dirty: bool,
}

impl Atlas {
    /// 新しいアトラスを作成する。`initial_size` は一辺のピクセル数（512 推奨）。
    pub fn new(device: &wgpu::Device, initial_size: u32) -> Self {
        let size = initial_size;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sdit glyph atlas"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            id: next_atlas_id(),
            data: vec![0u8; (size * size * 4) as usize],
            size,
            shelves: Vec::new(),
            next_shelf_y: 0,
            texture,
            texture_view,
            dirty: false,
        }
    }

    /// `width × height` ピクセルの領域を確保して `AtlasRegion` を返す。
    ///
    /// アトラスに収まらない場合は `None`。
    pub fn reserve(&mut self, width: u32, height: u32) -> Option<AtlasRegion> {
        if width == 0 || height == 0 {
            return None;
        }
        if width > self.size || height > self.size {
            return None;
        }

        // 同じ高さのシェルフを探す（高さが一致または余裕がある最小シェルフ）。
        // 簡易実装: 高さが height 以上で最も幅の余裕があるシェルフを探す。
        let shelf_idx =
            self.shelves.iter().position(|s| s.height >= height && s.cursor_x + width <= self.size);

        if let Some(idx) = shelf_idx {
            let s = &mut self.shelves[idx];
            let region = AtlasRegion { x: s.cursor_x, y: s.y, width, height };
            s.cursor_x += width;
            return Some(region);
        }

        // 新しいシェルフを作る。
        if self.next_shelf_y + height > self.size {
            return None; // アトラス満杯
        }

        let region = AtlasRegion { x: 0, y: self.next_shelf_y, width, height };
        self.shelves.push(Shelf { y: self.next_shelf_y, height, cursor_x: width });
        self.next_shelf_y += height;

        Some(region)
    }

    /// `region` に `data`（RGBA ピクセル列、4 bytes/pixel）を書き込む。
    ///
    /// `data.len()` は `region.width * region.height * 4` と等しくなければならない。
    pub fn write(&mut self, region: AtlasRegion, data: &[u8]) {
        let expected = (region.width * region.height * 4) as usize;
        if data.len() != expected {
            log::error!(
                "atlas::write: data length mismatch (expected {expected}, got {})",
                data.len()
            );
            return;
        }
        for row in 0..region.height {
            let dst_start = ((region.y + row) * self.size * 4 + region.x * 4) as usize;
            let src_start = (row * region.width * 4) as usize;
            let row_bytes = (region.width * 4) as usize;
            self.data[dst_start..dst_start + row_bytes]
                .copy_from_slice(&data[src_start..src_start + row_bytes]);
        }
        self.dirty = true;
    }

    /// ダーティフラグが立っているとき、CPU データを GPU テクスチャに転送する。
    pub fn upload_if_dirty(&mut self, queue: &wgpu::Queue) {
        if !self.dirty {
            return;
        }
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.size * 4),
                rows_per_image: Some(self.size),
            },
            wgpu::Extent3d { width: self.size, height: self.size, depth_or_array_layers: 1 },
        );
        self.dirty = false;
    }

    /// この Atlas の一意識別子を返す。
    pub fn id(&self) -> AtlasId {
        self.id
    }

    /// GPU テクスチャビューの参照を返す。
    pub fn texture_view(&self) -> &wgpu::TextureView {
        &self.texture_view
    }

    /// アトラスの一辺（ピクセル数）を返す。
    pub fn size(&self) -> u32 {
        self.size
    }

    /// アトラスをクリアする。全グリフ領域を解放し、GPU へのアップロードを予約する。
    ///
    /// グリフキャッシュを持つ `FontContext` の `glyph_cache` も合わせてクリアすること。
    pub fn clear(&mut self) {
        self.shelves.clear();
        self.next_shelf_y = 0;
        self.data.fill(0);
        self.dirty = true;
    }
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// wgpu デバイスが不要なインメモリテスト用のダミーアトラス。
    /// テクスチャ操作なしで reserve/write のロジックだけを検証する。
    /// データは RGBA 4 bytes/pixel。
    struct InMemAtlas {
        data: Vec<u8>,
        size: u32,
        shelves: Vec<Shelf>,
        next_shelf_y: u32,
    }

    impl InMemAtlas {
        fn new(size: u32) -> Self {
            Self {
                data: vec![0u8; (size * size * 4) as usize],
                size,
                shelves: Vec::new(),
                next_shelf_y: 0,
            }
        }

        fn reserve(&mut self, width: u32, height: u32) -> Option<AtlasRegion> {
            if width == 0 || height == 0 || width > self.size || height > self.size {
                return None;
            }
            let shelf_idx = self
                .shelves
                .iter()
                .position(|s| s.height >= height && s.cursor_x + width <= self.size);
            if let Some(idx) = shelf_idx {
                let s = &mut self.shelves[idx];
                let region = AtlasRegion { x: s.cursor_x, y: s.y, width, height };
                s.cursor_x += width;
                return Some(region);
            }
            if self.next_shelf_y + height > self.size {
                return None;
            }
            let region = AtlasRegion { x: 0, y: self.next_shelf_y, width, height };
            self.shelves.push(Shelf { y: self.next_shelf_y, height, cursor_x: width });
            self.next_shelf_y += height;
            Some(region)
        }

        fn write(&mut self, region: AtlasRegion, data: &[u8]) {
            let expected = (region.width * region.height * 4) as usize;
            if data.len() != expected {
                return;
            }
            for row in 0..region.height {
                let dst_start = ((region.y + row) * self.size * 4 + region.x * 4) as usize;
                let src_start = (row * region.width * 4) as usize;
                let row_bytes = (region.width * 4) as usize;
                self.data[dst_start..dst_start + row_bytes]
                    .copy_from_slice(&data[src_start..src_start + row_bytes]);
            }
        }

        fn clear(&mut self) {
            self.shelves.clear();
            self.next_shelf_y = 0;
            self.data.fill(0);
        }
    }

    #[test]
    fn reserve_returns_non_overlapping_regions() {
        let mut atlas = InMemAtlas::new(32);
        let r1 = atlas.reserve(8, 8).unwrap();
        let r2 = atlas.reserve(8, 8).unwrap();
        // 同じ高さなので同じシェルフに並ぶ。
        assert_eq!(r1.y, r2.y);
        assert_eq!(r1.x + r1.width, r2.x);
    }

    #[test]
    fn reserve_new_shelf_when_row_full() {
        let mut atlas = InMemAtlas::new(16);
        let r1 = atlas.reserve(10, 8).unwrap();
        let r2 = atlas.reserve(10, 8).unwrap(); // 幅不足 → 新しいシェルフ
        // 2つ目は新シェルフになるはず（y が異なる）
        assert_ne!(r1.y, r2.y);
    }

    #[test]
    fn reserve_fails_when_full() {
        let mut atlas = InMemAtlas::new(8);
        // 8x8 アトラスを全部使う。
        let _ = atlas.reserve(8, 8).unwrap();
        // 次の reserve は満杯で失敗。
        assert!(atlas.reserve(1, 1).is_none());
    }

    #[test]
    fn clear_resets_atlas() {
        let mut atlas = InMemAtlas::new(32);
        // 領域を確保してデータを書き込む（RGBA: 8x8x4 = 256 bytes）
        let region = atlas.reserve(8, 8).unwrap();
        atlas.write(region, &[255u8; 256]);
        assert!(atlas.data.iter().any(|&b| b != 0));

        // クリア後はゼロに戻り、新規確保できる
        atlas.clear();
        assert!(atlas.data.iter().all(|&b| b == 0), "クリア後のデータはすべてゼロ");
        assert!(atlas.shelves.is_empty(), "シェルフがクリアされていない");
        assert_eq!(atlas.next_shelf_y, 0, "next_shelf_y がリセットされていない");

        // クリア後も正常に領域確保できる
        let region2 = atlas.reserve(8, 8);
        assert!(region2.is_some(), "クリア後に reserve が失敗した");
    }

    // smell-allow: magic-number, assertion-roulette — ピクセルデータは連番パターンで意図が明確。コメントで各ピクセル位置を説明済み
    #[test]
    fn write_stores_rgba_pixels_correctly() {
        let mut atlas = InMemAtlas::new(16);
        let region = atlas.reserve(2, 2).unwrap();
        // 2x2 RGBA: 4 pixels × 4 bytes = 16 bytes
        // pixel (0,0): [10, 20, 30, 40]
        // pixel (1,0): [50, 60, 70, 80]
        // pixel (0,1): [90, 100, 110, 120]
        // pixel (1,1): [130, 140, 150, 160]
        let data = [
            10u8, 20, 30, 40, // pixel (0,0)
            50, 60, 70, 80, // pixel (1,0)
            90, 100, 110, 120, // pixel (0,1)
            130, 140, 150, 160, // pixel (1,1)
        ];
        atlas.write(region, &data);
        // (0,0) の R チャンネル = 10
        let base = ((region.y * atlas.size + region.x) * 4) as usize;
        assert_eq!(atlas.data[base], 10);
        assert_eq!(atlas.data[base + 1], 20);
        assert_eq!(atlas.data[base + 2], 30);
        assert_eq!(atlas.data[base + 3], 40);
        // (1,0) の先頭 = base + 4
        assert_eq!(atlas.data[base + 4], 50);
        // (0,1) の先頭 = base + size*4
        let row1_base = base + (atlas.size * 4) as usize;
        assert_eq!(atlas.data[row1_base], 90);
        assert_eq!(atlas.data[row1_base + 1], 100);
    }

    #[test]
    fn write_rejects_wrong_size() {
        let mut atlas = InMemAtlas::new(16);
        let region = atlas.reserve(2, 2).unwrap();
        // 正しいサイズは 2*2*4 = 16。古い R8 サイズ(4)は拒否される。
        let old_data = [10u8, 20, 30, 40]; // 4 bytes: 昔の R8 サイズ
        atlas.write(region, &old_data);
        // データが書き込まれていないはず（先頭はゼロのまま）
        let base = ((region.y * atlas.size + region.x) * 4) as usize;
        assert_eq!(atlas.data[base], 0, "wrong-size write should be rejected");
    }
}
