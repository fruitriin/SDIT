# Phase 17.1: テーマプリセット

**概要**: 複数のカラーテーマを名前付きプリセットとして定義し、設定やキーバインドで切り替えられるようにする。

**状態**: 未着手

## 背景

- 現在はカラーパレットを個別に設定できるが、テーマとして一括切り替えする仕組みがない
- 多くのターミナルエミュレータは built-in テーマや外部テーマファイルをサポートしている

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| 組み込みテーマ定義 | TOML 形式で 5〜10 個の定番テーマを定義（Solarized Dark/Light, Dracula, Nord, One Dark, Gruvbox, Tokyo Night 等） | sdit-core (`config/themes/`) | 未着手 |
| テーマ選択設定 | `theme = "solarized-dark"` で組み込みテーマを選択 | sdit-core (`config/mod.rs`) | 未着手 |
| カスタムテーマファイル | `~/.config/sdit/themes/` からユーザー定義テーマを読み込み | sdit-core (`config/mod.rs`) | 未着手 |
| テーマ切り替えアクション | `Action::NextTheme` / `Action::PreviousTheme` でサイクル切り替え | sdit-core (`config/keybinds.rs`), sdit (`event_loop.rs`) | 未着手 |
| Hot Reload 連携 | テーマ変更時にカラーパレットを即座に反映 | sdit (`event_loop.rs`) | 未着手 |
| テスト | テーマ読み込み・切り替え・フォールバック | sdit-core | 未着手 |

## 設定例

```toml
[colors]
theme = "solarized-dark"  # 組み込みテーマ名 or カスタムテーマファイル名
```

カスタムテーマファイル `~/.config/sdit/themes/my-theme.toml`:
```toml
[palette]
foreground = "#d4d4d4"
background = "#1e1e1e"
# ... 16色パレット
```

## 参照

- `refs/ghostty/src/config/Config.zig` — theme
- `refs/alacritty/alacritty/src/config/color.rs` — Colors
