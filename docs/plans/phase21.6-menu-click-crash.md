# Phase 21.6: メニュークリック時クラッシュの修正

## 問題

macOS でメニューバー項目（File > New Window 等）をクリックすると SDIT がクラッシュする。

## 症状

```
thread 'main' panicked at muda-0.17.1/src/platform_impl/macos/icon.rs:34:53:
called `Result::unwrap()` on an `Err` value: Format(FormatError { inner: ZeroWidth })
```

## 原因

`muda 0.17.1` の `MenuItem::fire_menu_item_click()` 内で `PlatformIcon::to_png()` が呼ばれ、
ゼロ幅のアイコン（アイコン未設定時の 0x0 PlatformIcon）を PNG エンコードしようとして
`unwrap()` がパニックする。

スタックトレース:
```
muda::platform_impl::platform::icon::PlatformIcon::to_png
muda::platform_impl::platform::icon::PlatformIcon::to_nsimage
muda::platform_impl::platform::MenuItem::fire_menu_item_click
muda::platform_impl::platform::MenuItem::fire_menu_item_action
```

## 発見経緯

Phase 21.5 統合テスト（`docs/test-scenarios/028-macos-menubar.md`）の Part 3
「File > New Window クリック」で再現。

## 修正方針

以下のいずれかで対応:

### 案 A: muda をアップグレードする（推奨）
`muda` の最新バージョンを確認し、このバグが修正済みかを確認する。
修正済みであれば `Cargo.toml` のバージョンを更新する。

### 案 B: アイコンを設定する
各 `MenuItem` にダミーアイコンまたは実際のアイコンを設定し、
ゼロ幅アイコンの発生を防ぐ。

### 案 C: muda をフォークして修正する
`PlatformIcon::to_png()` 内の `unwrap()` を `?` に変え、
エラー時はアイコンなしでクリックを処理する。

## 変更対象

- `crates/sdit/Cargo.toml` — muda バージョン更新（案 A の場合）
- `crates/sdit/src/menu.rs` — MenuItem アイコン設定（案 B の場合）

## テスト

- `docs/test-scenarios/028-macos-menubar.md` Part 3
  - `click-menu.sh sdit "File" "New Window"` でクラッシュしないこと
  - `click-menu.sh sdit "File" "New Tab"` でクラッシュしないこと

## 実装結果（2026-03-15 完了）

### 根本原因（調査で判明）

当初の原因分析は表面的だった。本当の原因は:

1. **dangling ポインタ（主因）**: `event_loop.rs` の `self.menu_bar.take()` で `muda::Menu` をドロップすると、
   NSMenuItem の ObjC `ivars` に保持されていた `*const MenuChild` 生ポインタが dangling になる。
   メニュークリック時に `fire_menu_item_click` がこの無効ポインタを経由して `predefined_item_type` を読み取り、
   ガベージデータを `Some(PredefinedMenuItemType::About(about_meta))` として解釈して
   About panel コードに入り、最終的に `to_png()` または `NSString::from_str()` でパニック。

2. **ゼロ幅アイコンパニック（従因）**: `PlatformIcon::to_png()` がゼロ幅でパニックするのは防御的ガードの欠如。

### 実装した修正

#### 主修正: `crates/sdit/src/event_loop.rs`
`self.menu_bar.take()` → `self.menu_bar.as_ref()` に変更。
`Menu` を `SditApp` が存在する間保持し続けることで dangling ポインタを根本解消。

#### 防御修正: `vendor/muda-0.17.1/`（Case C 相当のローカルパッチ）
- `icon.rs`: `to_png()` にゼロ幅ガード追加
- `mod.rs`: `fire_menu_item_click` と `menuitem_set_icon` にゼロサイズアイコンスキップを追加

#### 依存パッチ: `Cargo.toml`
`[patch.crates-io]` で `vendor/muda-0.17.1` を使用。

### テスト結果
- File > New Window / New Tab / Close Tab、View > Zoom In 等 全操作でクラッシュなし確認 ✓
- セキュリティレビュー: Critical/High/Medium/Low 0件 ✓

## セキュリティ影響

なし（セキュリティレビュー済み、問題なし）
