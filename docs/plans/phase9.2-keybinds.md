# Phase 9.2: キーバインドカスタマイズ

**概要**: TOML設定ファイルからキーバインドを定義・変更可能にする。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先クレート | 状態 |
|---|---|---|---|
| Action enum | 15アクション（NewWindow, Copy, Paste, Search等）を enum 化 | sdit-core (`config/keybinds.rs`) | 完了 |
| KeyBinding + KeybindConfig | TOML `[[keybinds]]` スキーマ + デフォルトバインド（macOS/Linux） | sdit-core (`config/keybinds.rs`) | 完了 |
| Config 統合 | `Config.keybinds` フィールド追加 + validate() | sdit-core (`config/mod.rs`) | 完了 |
| resolve_action | key+mods → Action ルックアップ（キャッシュ済みビットフィールド） | sdit (`input.rs`) | 完了 |
| event_loop リファクタリング | `is_*_shortcut()` 関数群を Action-based dispatch に置換 | sdit (`event_loop.rs`) | 完了 |

## 依存関係

Phase 8

## リファレンス

- `refs/alacritty/alacritty/src/config/bindings.rs` — キーバインド設定の型定義
- `refs/alacritty/alacritty/src/input/keyboard.rs` — バインディング実行時ルックアップ

## セキュリティレビュー結果

### 修正済み (Medium)

- **M-1**: `key`/`mods` フィールド長制限（64文字）— `KeybindConfig::validate()` で除外
- **M-2**: バインディング件数上限（512件）— `validate()` で切り捨て
- **M-3**: `parse_mods` 毎回実行問題 — `cached_mods_bits` ビットフィールドでロード時にキャッシュ、実行時は整数比較のみ

### 記録のみ (Low / Info)

- **L-1**: `to_lowercase()` バイト長膨張によるスライスパニックの可能性（Phase 9.1 由来）
- **L-2**: `shifted_equivalent` の双方向マッチ — 同一物理キーなので実害なし
- **L-3**: `update_search` 内の Mutex 保持構造（Phase 9.1 由来）
- **L-4**: Action の TOML デシリアライズが PascalCase のみ対応
- **I-1**: `HashSet` フレームごと再構築（Phase 9.1 由来）
- **I-2**: 検索バー `overwrite_cell` 境界確認依存（Phase 9.1 由来）
