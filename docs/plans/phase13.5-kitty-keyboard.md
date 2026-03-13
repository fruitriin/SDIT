# Phase 13.5: Kitty Keyboard Protocol

**概要**: neovim 等が要求する拡張キーボードプロトコル。修飾キーの正確な報告、キーリリースイベント等を提供。

**状態**: **完了**（disambiguate フラグのサポート）

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| KittyKeyboardFlags | 5ビットプログレッシブエンハンスメントフラグ | sdit-core (`terminal/mod.rs`) | 完了 |
| KittyFlagStack | 8エントリ固定サイズスタック（push/pop/set/current） | sdit-core (`terminal/mod.rs`) | 完了 |
| CSI ハンドラ | `CSI > u` push / `CSI < u` pop / `CSI ? u` query | sdit-core (`terminal/handler.rs`) | 完了 |
| CSI u エンコーディング | Kitty キーマッピング + CSI code;mods u 形式出力 | sdit (`input.rs`) | 完了 |
| key_to_bytes 改修 | Kitty/レガシーディスパッチ | sdit (`input.rs`) | 完了 |
| event_loop 統合 | kitty_flags を key_to_bytes に渡す | sdit (`event_loop.rs`) | 完了 |
| テスト | FlagStack 6件 + エンコーディング 12件 + CSIハンドラ 2件 | sdit-core, sdit | 完了 |

## 対応範囲

- **フラグ1（disambiguate）**: 完全サポート。CSI u エンコーディング、修飾キー付き報告
- **フラグ2-5（report_events, report_alternates, report_all, report_associated）**: フラグスタックで保持するが、エンコーディングは未実装。将来フェーズで対応予定

## 参照

- `refs/ghostty/src/input/key_encode.zig`
- `refs/ghostty/src/terminal/kitty/key.zig`

## 依存関係

なし

## セキュリティレビュー結果

### M-1: CSI push パラメータのキャスト精度損失（Medium）— 修正済み

u16→u8 キャストで不正フラグ値がバイパスされる。

**修正**: u16 のまま `& 0x1f` でクランプ後にキャスト。

### M-2: CSI pop パラメータに上限なし（Medium）— 修正済み

**修正**: `.clamp(1, 8)` で制限。

### L-1: kitty_csi の suffix パラメータ（Low）

char 型で制御文字を許容し得る。現在は固定値のみ渡されるため実害なし。

### L-2: kitty_flags フィールドの公開範囲（Low）

`pub` だが `pub(crate)` が適切。

### L-3: Alt + マルチバイト文字の消失（Low）

Kitty モード時に Alt + マルチバイト文字が None を返す。レガシーフォールバックの検討が必要。

### I-1: FlagStack オーバーフロー時の動作（Info）— 修正済み

**修正**: 満杯時は push を無視（最古破棄ではなくサイレント失敗）。
