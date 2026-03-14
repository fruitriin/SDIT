# SDIT

> SDIファースト、縦タブセカンドのターミナルエミュレータ

```
セッションは本来バラバラに存在する。
束ねたくなったときだけ縦タブが出現する。
```

各タブはドラッグアンドドロップで統合・分割する。
（Chrome like UX）

## 特徴

- **SDI ファースト**: 1ウィンドウ = 1セッションがデフォルト。タブバーは2つ以上のセッションがあるときだけ出現する
- **GPU レンダリング**: wgpu による高速描画。リガチャ・絵文字・CJK 全角文字対応
- **VTE 互換**: ANSIエスケープシーケンス、SGR（色・装飾）、OSC（タイトル・クリップボード・通知）、CSI（カーソル・スクロール）を幅広くサポート
- **TOML 設定 + ホットリロード**: 設定ファイルの変更を即座に反映。GUI 設定画面からも編集可能
- **テキスト選択 & クリップボード**: マウスドラッグ選択、ダブルクリックで単語選択、トリプルクリックで行選択
- **検索**: Cmd+F でターミナル出力内を検索。マッチハイライト表示
- **URL 検出 & クリック**: Cmd+クリックでURLをブラウザで開く
- **IME 入力**: 日本語などのマルチバイト文字入力に対応
- **セッション復元**: ウィンドウ位置・セッション情報を保存し、次回起動時に復元
- **macOS ネイティブ統合**: メニューバー、コンテキストメニュー、.app バンドル対応
- **カスタムキーバインド**: TOML で自由にキーバインドを設定可能
- **テーマ切り替え**: 複数のカラーテーマをサイクル切り替え
- **シェルインテグレーション**: OSC 133 によるプロンプト認識・コマンド完了通知

## オプトイン機能

設定ファイル（`~/.config/sdit/config.toml`）で有効化できる機能:

| 機能 | 設定キー | 説明 |
|---|---|---|
| 背景ブラー | `window.blur = true` | macOS でウィンドウ背景にブラーエフェクト |
| 常時最前面 | `window.always_on_top = true` | ウィンドウを常に最前面に表示 |
| 背景画像 | `window.background_image = "~/path.png"` | ターミナル背景に画像を表示 |
| カーソル点滅 | `cursor.blinking = true` | カーソルの点滅を有効化 |
| 選択時の自動コピー | `selection.save_to_clipboard = true` | テキスト選択完了時に自動クリップボードコピー |
| タイピング中マウス非表示 | `mouse.hide_when_typing = true` | タイピング中にマウスカーソルを隠す |
| Focus Follows Mouse | `mouse.focus_follows_mouse = true` | マウスホバーで自動フォーカス |
| 出力時のスクロール復帰 | `scrolling.scroll_to_bottom_on_output = true` | 新しい出力があると自動で最下部へ |
| Quick Terminal | `quick_terminal.enabled = true` | グローバルホットキーでスライドインするターミナル（macOS） |
| Secure Keyboard Entry | `security.auto_secure_input = true` | フォーカス時にセキュアキーボード入力を自動有効化（macOS） |
| タイトル報告 | `terminal.title_report = true` | CSI 21t によるウィンドウタイトルの報告を許可 |

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
