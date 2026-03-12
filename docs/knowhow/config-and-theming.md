# TOML 設定・カラーテーマ・CJK 対応の知見

## TOML 設定基盤

- `#[serde(default)]` を struct と Vec フィールドの両方に付けることで、設定ファイルの部分指定に対応
- `Config::load()` は `fs::read_to_string` + `toml::from_str` のシンプルな構成。ファイル不在・パース失敗時はデフォルト値にフォールバック
- `dirs::config_dir()` で XDG 準拠のパスを解決（macOS: `~/Library/Application Support`、Linux: `~/.config`）

## f32 設定値の安全なクランプ

- `f32::clamp()` は NaN に対して NaN を返す仕様 → `is_finite()` チェックを先に行う必要がある
- パターン: `if value.is_finite() { value.clamp(min, max) } else { default }`

## カラーテーマ

- `ResolvedColors` は f32 RGBA で統一。GPU シェーダーに直接渡せる
- `hex_to_rgba()` ヘルパーで `u8` → `f32` 変換。clippy pedantic では `f32::from(u8)` を要求される
- WCAG 2.1 コントラスト比計算: sRGB → 線形化 → 相対輝度 → (L1 + 0.05) / (L2 + 0.05)
- 全テーマで WCAG AA (4.5:1) と dim (3:1) のテストを回すことで、テーマ追加時の退行を防ぐ

## CJK 全角文字の描画

- sdit-core の `CellFlags::WIDE_CHAR` / `WIDE_CHAR_SPACER` を pipeline.rs で参照
- `WIDE_CHAR_SPACER` セルは背景のみ描画（グリフなし）
- `WIDE_CHAR` セルには `cell_width_scale: 2.0` を設定し、シェーダー側でクワッドを2セル幅に拡張
- シェーダー内で `clamp(cell_width_scale, 1.0, 2.0)` して異常値を防御

## 設定 Hot Reload（config_watcher）

- `notify::RecommendedWatcher` でファイルの **親ディレクトリ** を監視する（ファイル自体の監視は一部 FS で動作しないため）
- デバウンス 300ms: `Instant` を `Arc<Mutex>` で共有し、最後のイベントから 300ms 経過後に `SditEvent::ConfigReloaded` を送信する
- 親ディレクトリが存在しない場合は `None` を返して起動を続行する（graceful fallback）
  - 例: `~/.config/sdit/` ディレクトリが未作成の場合は WARN ログを出して監視なしで起動する
- `_watcher` を `main.rs` 内でドロップせずに保持する必要がある（ドロップすると監視が停止する）
- `apply_config_reload()` は `Config::load()` でファイルを再読み込みし、フォント・カラー・キーバインドの差分のみ更新する
  - フォント変更時: `FontContext` を再構築 + 全アトラスクリア + 全セッション resize
  - カラー変更時: `ResolvedColors` を差し替え
  - キーバインド変更時: `config` を置換（validate は load 内で実施済み）

## 永続化のアトミック書き込み

- 一時ファイル + rename パターンでデータ破損を防ぐ
- 一時ファイル名に PID + ナノ秒タイムスタンプを含めて TOCTOU 攻撃を軽減
- rename 失敗時は一時ファイルをクリーンアップ
