//! テクスチャアトラス — グリフビットマップを1枚のテクスチャに詰め込む。
//!
//! シェルフアルゴリズムで矩形を配置する。同じ高さのグリフを横に並べてシェルフを構成し、
//! シェルフが満杯になったら新しいシェルフを作る。
//!
//! テクスチャフォーマットは `R8Unorm`（グレースケール Alpha マスク）。

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

/// `R8Unorm` テクスチャアトラス。
///
/// CPU 側の `data` バッファに書き込み、`upload_if_dirty` で GPU テクスチャに転送する。
pub struct Atlas {
    /// CPU ピクセルデータ（R8: 1 byte/pixel）。
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
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            data: vec![0u8; (size * size) as usize],
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

    /// `region` に `data`（R8 ピクセル列）を書き込む。
    ///
    /// `data.len()` は `region.width * region.height` と等しくなければならない。
    pub fn write(&mut self, region: AtlasRegion, data: &[u8]) {
        let expected = (region.width * region.height) as usize;
        if data.len() != expected {
            log::error!(
                "atlas::write: data length mismatch (expected {expected}, got {})",
                data.len()
            );
            return;
        }
        for row in 0..region.height {
            let dst_start = ((region.y + row) * self.size + region.x) as usize;
            let src_start = (row * region.width) as usize;
            let src_end = src_start + region.width as usize;
            self.data[dst_start..dst_start + region.width as usize]
                .copy_from_slice(&data[src_start..src_end]);
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
                bytes_per_row: Some(self.size),
                rows_per_image: Some(self.size),
            },
            wgpu::Extent3d { width: self.size, height: self.size, depth_or_array_layers: 1 },
        );
        self.dirty = false;
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
    struct InMemAtlas {
        data: Vec<u8>,
        size: u32,
        shelves: Vec<Shelf>,
        next_shelf_y: u32,
    }

    impl InMemAtlas {
        fn new(size: u32) -> Self {
            Self {
                data: vec![0u8; (size * size) as usize],
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
            for row in 0..region.height {
                let dst_start = ((region.y + row) * self.size + region.x) as usize;
                let src_start = (row * region.width) as usize;
                self.data[dst_start..dst_start + region.width as usize]
                    .copy_from_slice(&data[src_start..src_start + region.width as usize]);
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
        // 領域を確保してデータを書き込む
        let region = atlas.reserve(8, 8).unwrap();
        atlas.write(region, &[255u8; 64]);
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

    #[test]
    fn write_stores_pixels_correctly() {
        let mut atlas = InMemAtlas::new(16);
        let region = atlas.reserve(2, 2).unwrap();
        atlas.write(region, &[10, 20, 30, 40]);
        // (0,0) = 10, (1,0) = 20, (0,1) = 30, (1,1) = 40
        assert_eq!(atlas.data[(region.y * atlas.size + region.x) as usize], 10);
        assert_eq!(atlas.data[(region.y * atlas.size + region.x + 1) as usize], 20);
        assert_eq!(atlas.data[((region.y + 1) * atlas.size + region.x) as usize], 30);
    }
}
