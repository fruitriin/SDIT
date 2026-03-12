# テキスト選択 + クリップボード実装ノウハウ

## Selection 型の設計

### SelectionType と正規化

- `Simple`（ドラッグ）、`Word`（ダブルクリック）、`Lines`（トリプルクリック）の3種
- `start > end` のドラッグ逆方向を `normalized()` で正規化する
- `Point` の `PartialOrd` は `(line, column)` 辞書順なので `self.start <= self.end` で正規化判定できる

### `contains()` 実装

- `Lines` タイプは行全体なので列チェックをスキップ
- 複数行をまたぐ場合: 開始行は `col >= sc`、終了行は `col <= ec`、中間行は全列

### `to_tuple()` によるパイプライン互換

- `pipeline.rs` の `is_in_selection()` は `((usize, usize), (usize, usize))` タプルを受け取る
- `to_tuple(grid_cols)` で変換して渡すことで pipeline.rs の変更不要

## ダブル/トリプルクリック判定

```rust
// app.rs のフィールド
last_click_time: Option<Instant>
last_click_pos: Option<(usize, usize)>
click_count: u8  // max 3
```

- 400ms 以内・同位置クリックで `click_count` をインクリメント（最大3）
- `click_count` が 2 → Word、3 → Lines

## 単語選択（expand_word）

- 区切り文字（空白・記号）の同類を左右に広げる
- 起点セルの文字が区切り文字かどうかを `origin_is_sep` で判定し、同じカテゴリで拡張

## クリップボード統合（arboard）

- `arboard::Clipboard::new()` の失敗は `Option<Clipboard>` で吸収（headless 環境等）
- `arboard` はメインスレッドでの使用が必須（macOS AppKit 制約）
- `SditApp` のフィールドに持ち、イベントハンドラから直接使用する

## OSC 52 クリップボード書き込み

- `\e]52;c;<base64>\a` を受信したらデコードして `clipboard_write_pending` に格納
- `\e]52;c;?` 読み取り要求には応答しない（セキュリティ）
- PTY リーダースレッドで `take_clipboard_write()` をチェック → `SditEvent::ClipboardWrite(text)` をメインスレッドに送信
- メインスレッドの `user_event()` で arboard に書き込む

### Base64 デコード

依存クレートを増やさないため自前実装:
- 256要素の lookup table で O(1) マッピング
- 不正文字 → `None` を返す
- `=` padding で停止

## BRACKETED_PASTE

```rust
if mode.contains(TermMode::BRACKETED_PASTE) {
    // \e[200~ + text + \e[201~
}
```

- vim/neovim 等のブラケットペーストモードに対応
- ペースト内容が誤ってコマンドとして解釈されるのを防ぐ

## セキュリティ注意事項

- OSC 52 書き込みは 1 MiB で上限制限（`MAX_CLIPBOARD_BYTES`）
- 将来 config で `allow_osc52_write: bool` を追加することを検討
