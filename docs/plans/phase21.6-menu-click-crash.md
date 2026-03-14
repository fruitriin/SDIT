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

## セキュリティ影響

なし
