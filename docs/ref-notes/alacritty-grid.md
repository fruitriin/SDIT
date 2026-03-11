# Alacritty: グリッド・VTE・PTY 読解メモ

## 対象ファイル
- `refs/alacritty/alacritty-terminal/src/grid/` — グリッドデータ構造
- `refs/alacritty/alacritty-terminal/src/ansi.rs` — VTEパーサー統合
- `refs/alacritty/alacritty-terminal/src/tty/` — PTYプロセス管理

---

## 1. グリッド設計

### Row<T> と Cell の構造

- **Row<T>**: `Vec<T>` のラッパー（newtype）
  - `inner: Vec<T>` — セルデータ
  - `occ: usize` — 最後に変更されたセルの上限インデックス（dirty tracking用）
  - `grow()` / `shrink()` でカラム数変更対応

- **Cell**: 単一セルの内容と属性
  - `c: char` — 表示文字
  - `fg: Color`, `bg: Color` — 前景・背景色
  - `flags: Flags` — テキスト属性（太字、斜体、下線など）
  - `extra: Option<Arc<CellExtra>>` — ゼロ幅文字、下線色、ハイパーリンク（遅延割り当て）

### Storage<T> — リングバッファ実装

スクロールバック高速化のため、物理的なメモリ移動を避けるリングバッファ:

- `inner: Vec<Row<T>>` — 実メモリ
- `zero: usize` — リングバッファ開始位置（rotation offset）
- `len: usize` — 使用中の行数
- `visible_lines: usize` — ビューポート行数

主要操作:
- `rotate(count)` — `zero` の加算のみで全行シフト（O(1)）
- `compute_index()` — 論理行番号 → 物理配列インデックス変換
- `rezero()` — メモリコンパクト化
- `MAX_CACHE_SIZE = 1000` 以上の未使用バッファは自動トリミング

### Grid<T> のメモリレイアウト

```
┌─────────────────────────────┐  max_scroll_limit + lines
│      未初期化領域           │
├─────────────────────────────┤  raw.inner.len()
│      リサイズ用バッファ      │
├─────────────────────────────┤  history_size() + lines
│    スクロールバック領域      │
├─────────────────────────────┤  lines
│    表示中の領域（ビューポート）│  ← display_offset で参照位置指定
├─────────────────────────────┤  0
│    スクロールダウン領域       │
└─────────────────────────────┘
```

Grid メンバー:
- `cursor: Cursor<T>` — カーソル位置・属性・charset
- `saved_cursor: Cursor<T>` — DECSC/DECRC用
- `raw: Storage<T>` — リングバッファ
- `display_offset: usize` — スクロールバック表示位置（0=最下部）
- `max_scroll_limit: usize` — スクロールバック上限行数

### Alternate Screen Buffer

- `grid` と `inactive_grid` を入れ替えて Primary/Alternate を切り替え
- pager/vim 使用時に活用

---

## 2. VTEパーサー統合

### vte クレートの使い方

- `ansi::Processor` が vte パーサーをラップ
- `advance(&mut handler, buf: &[u8])` でバイト列を処理
- Handler trait のメソッドをコールバック形式で呼び出し

### Handler trait 実装パターン

`Term<T: EventListener>` が Handler を実装。主要メソッド:

- `input(c: char)` — 通常文字の出力（折り返し・全角判定含む）
- `goto(line, col)` — カーソル移動（CUP）
- `move_up/down/forward/backward()` — 相対カーソル移動
- `insert_blank(count)` — 空白挿入
- `identify_terminal()` — DA応答
- 計70以上のメソッド

### ANSIシーケンス処理フロー

```
PTY読み込み → Parser::advance() → Handler メソッド呼び出し
  → Grid/Term 状態更新 → Damage tracking
```

### Damage Tracking

- `TermDamageState` — 各行の左右の変更範囲を記録
- `damage_line(line, left, right)` で最小矩形を拡張
- レンダラーは変更領域だけを再描画

---

## 3. PTYプロセス管理

### PTY 生成フロー

1. `openpty()` で PTY pair 生成（kernel PTY）
2. `Command::new(shell)` でシェルコマンド構築
3. `stdin/stderr/stdout` に PTY slave を接続
4. `pre_exec` フックで `setsid()`, 制御端末設定, シグナルリセット
5. `spawn()` & master fd の nonblocking 化

### I/O 処理パターン

- `polling` クレート（epoll/kqueue 抽象）による非同期I/O
- イベントループ:
  1. チャネルメッセージをドレイン（入力・リサイズ）
  2. PTY から非ブロッキング読み込み
  3. VTE パーサー実行
  4. PTY へ書き込み（キーボード入力）
  5. ポーリング待機

### バッファ管理

- `READ_BUFFER_SIZE = 0x10_0000` (1MB) — 1回の read ループ上限
- `MAX_LOCKED_READ = u16::MAX` — Terminal lock 保持時間制限
- 読み込み中に Terminal lock を頻繁に開放（GUI応答性確保）

### シグナル処理

- `signal-hook` で SIGCHLD を登録
- `UnixStream` パイプ経由で非同期通知
- `child.try_wait()` で終了ステータスポーリング

---

## SDITへの適用

### 採用する設計

| 要素 | 適用先 | 備考 |
|---|---|---|
| Row<T>/Cell 構造 | `sdit-core/src/grid/` | `occ` フィールドで dirty tracking |
| Storage<T> リングバッファ | `sdit-core/src/grid/` | スクロール O(1) |
| Grid メモリレイアウト | `sdit-core/src/grid/` | `display_offset` の管理 |
| Handler trait パターン | `sdit-core/src/terminal/` | `Term<T: EventListener>` |
| Damage tracking | `sdit-core/src/terminal/` | レンダラー差分描画に必須 |
| polling ベースの EventLoop | `sdit-session/` | PTY I/O 処理 |
| signal-hook + UnixStream | `sdit-session/` | SIGCHLD 処理 |

### 注意点

- `input_needs_wrap` — 行末到達時の改行遅延フラグ。テスト重要
- `display_offset > 0` 時の自動スクロール判定
- Cursor charset 管理 — 複数セッションで独立管理
- `MAX_CACHE_SIZE` の値はメモリ使用量とトレードオフ
- スクロールバック上限は TOML 設定で変更可能にする

### 採用しないもの

- OpenGL レンダラー（wgpu で独自実装）
- macOS/X11 ウィンドウ生成コード（winit で統一）
