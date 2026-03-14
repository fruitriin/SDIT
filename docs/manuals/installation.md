# インストール

## ビルド要件

- Rust toolchain（rustup 推奨）
- macOS: Xcode Command Line Tools

## ソースからビルド

```bash
# リポジトリをクローン
git clone https://github.com/riin/sdit.git
cd sdit

# リファレンスサブモジュールの取得（オプション、開発用）
git submodule update --init --depth=1

# ビルド
cargo build --release

# 実行
cargo run --release --bin sdit
```

## macOS .app バンドル

```bash
# .app バンドルを作成
make app

# Universal Binary（Intel + Apple Silicon）
make app-universal

# DMG を作成
make dmg
```

作成された .app バンドルは `target/release/bundle/SDIT.app` に出力されます。
Applications フォルダにコピーすれば Launchpad から起動できます。

```bash
cp -r target/release/bundle/SDIT.app /Applications/
```

## 初回セットアップ

SDIT は設定ファイルなしでも動作します。カスタマイズしたい場合は設定ファイルを作成してください。

```bash
mkdir -p ~/.config/sdit
touch ~/.config/sdit/config.toml
```

設定例:

```toml
[font]
family = "Menlo"
size = 16.0

[colors]
theme = "dracula"

[window]
opacity = 0.95
```
