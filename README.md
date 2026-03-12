# SDIT

> SDIファースト、縦タブセカンドのターミナルエミュレータ

```
セッションは本来バラバラに存在する。
束ねたくなったときだけ縦タブが出現する。
```

各タブはドラッグアンドドロップで統合・分割する。
（Chrome like UX）

## クイックスタート

```bash
# リファレンスサブモジュールの取得（浅いクローン）
git submodule update --init --depth=1

# ビルド
cargo build

# 実行
cargo run --bin sdit
```

## アーキテクチャ

```
crates/
  sdit/              バイナリ。GUIループ・ウィンドウ管理・イベントループ
  sdit-core/         ライブラリ。GUI非依存のコア機能をすべて集約
    terminal/          VTEステートマシン・ANSIシーケンス処理
    grid/              セルグリッド・スクロールバック
    pty/               PTYプロセス管理
    font/              フォントラスタライズ
    render/            wgpuレンダーパイプライン・テクスチャアトラス
    session/           セッション管理・サイドバー状態
    config/            TOML設定スキーマ・カラーテーマ
refs/                リファレンスOSS（git submodule, 読み取り専用）
docs/                読解メモ・計画ファイル
```

## リファレンスプロジェクト

| プロジェクト | 参照目的 |
|---|---|
| [Alacritty](https://github.com/alacritty/alacritty) | PTYコア・グリッド・VTEパーサー |
| [Ghostty](https://github.com/ghostty-org/ghostty) | コア/GUI分離アーキテクチャ・高速レンダリング |
| [WezTerm](https://github.com/wezterm/wezterm) | SDIウィンドウ管理・セッション状態 |
| [Zellij](https://github.com/zellij-org/zellij) | セッション管理・縦タブUI |

詳細は [CLAUDE.md](./CLAUDE.md) を参照。

## ライセンス

GPLv3
