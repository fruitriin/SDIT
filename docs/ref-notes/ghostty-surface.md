# Ghostty: サーフェス概念・コア/GUI分離 読解メモ

## 対象ファイル
- `refs/ghostty/src/Surface.zig` — サーフェス管理
- `refs/ghostty/src/App.zig` — アプリケーション構造
- `refs/ghostty/src/terminal/Terminal.zig` — ターミナルステートマシン

---

## 1. サーフェス管理（Surface.zig）

### Surface の定義と役割

> Surface represents a single terminal "surface". A terminal surface is a minimal "widget"
> where the terminal is drawn and responds to events such as keyboard and mouse.
> Each surface also creates and owns its pty session.

Surface は「描画先 + イベント処理 + 設定管理」の単位。何に表示されているか（ウィンドウ、タブ、分割ペイン）を気にしない。

### 主要フィールド

- `app: *App` — 親アプリケーション
- `rt_app, rt_surface` — ランタイム参照（GUI フレームワーク依存部）
- `renderer: Renderer` — wgpu ベースの描画パイプライン
- `io: termio.Termio` — PTY の読み書きハンドラ
- `io_thread` — 専用 OS スレッド（PTY I/O 処理）
- `renderer_thr` — 専用 OS スレッド（描画処理）
- `config: DerivedConfig` — Surface 専用の設定コピー

### Surface と Terminal の関係

- **Surface** = 描画先 + イベント処理
- **Terminal** = VTE パーサー + スクリーン状態 + グリッド
- Surface 内部の `io.terminal` が Terminal インスタンスを所有
- 1 Surface = 1 Terminal（Ghostty のモデル）

### Surface のライフサイクル

1. **生成（init）**: DerivedConfig 作成 → Renderer 初期化 → IO スレッド起動 → Renderer スレッド起動
2. **運用**: イベントハンドラで入力処理、io_thread が PTY 出力を Terminal に供給、renderer_thread が描画
3. **破棄（deinit）**: io_thread 停止・join → renderer_thread 停止・join → リソース解放

### コア/GUI 分離の実現

```
┌─────────────────────────────────────────────┐
│  GUI Layer (apprt.zig)                      │
│  - Window management, Platform-specific     │
└──────┬──────────────────────────────────────┘
       │ rt_app, rt_surface pointers
┌──────▼──────────────────────────────────────┐
│  Surface Layer                              │
│  - Input dispatching, Render thread mgmt    │
└──────┬──────────────────────────────────────┘
       │ owns io (Termio)
┌──────▼──────────────────────────────────────┐
│  Core Layer (headless)                      │
│  - Terminal, Termio, Renderer backend       │
│  ※ GUI dependencies ZERO                   │
└─────────────────────────────────────────────┘
```

apprt.zig の抽象インターフェースで GUI バックエンドを差し替え可能（gtk, none/headless）。

---

## 2. アプリケーション構造（App.zig）

### App と Surface の関係

```zig
App {
    surfaces: SurfaceList,          // *apprt.Surface の Vec
    focused_surface: ?*Surface,     // フォーカス中の Surface
    font_grid_set: SharedGridSet,   // フォント共有リソース
    mailbox: Mailbox.Queue,         // イベントキュー（容量64）
}
```

- App は複数 Surface を管理する親
- フォント `SharedGridSet` は Surface 間で共有
- 各 Surface は app への逆参照ポインタを保有（双方向）

### イベントループ — Mailbox パターン

```
tick() → drainMailbox():
  .open_config  → 設定画面を開く
  .new_window   → 新 Surface 作成
  .close        → Surface 破棄
  .surface_message → Surface へメッセージ転送
  .redraw_surface  → 再描画要求
  .quit         → アプリ終了
```

- IO/Renderer スレッドからのメッセージは Mailbox 経由で App メインスレッドへ
- ロックフリーの BlockingQueue で同期

### Surface 追加・削除

- `addSurface()`: リストに追加、Quit timer キャンセル
- `deleteSurface()`: focused 無効化（use-after-free 防止）、最後の Surface 削除時に Quit timer 開始

---

## 3. ターミナルステート（Terminal.zig）

### Terminal の状態管理

```zig
Terminal {
    screens: ScreenSet,             // Primary + Alternate
    rows, cols: CellCountInt,
    scrolling_region: ScrollingRegion,  // DECSTBM
    modes: ModeState,               // VT100 モード（64bit packed）
    colors: Colors,                 // パレット、OSC 10/11/12 対応
    flags: packed struct {
        mouse_event, mouse_format,
        focused, password_input,
        dirty: Dirty,               // レンダラー用
    },
}
```

### Screen/Grid の関係

```
ScreenSet
  ├─ active_key: .primary | .alternate
  ├─ active: *Screen（現在のスクリーン）
  └─ all: EnumMap(Key, *Screen)

Screen
  ├─ pages: PageList（スクロールバック）
  ├─ cursor: Cursor
  ├─ selection: ?Selection
  └─ dirty: Dirty
```

### Dirty Flags

```zig
Dirty {
    palette: bool,       // パレット変更
    reverse_colors: bool, // DECSCNM 変更
    clear: bool,         // ED/EL 実行
    preedit: bool,       // IME 入力中
}
```

---

## SDITへの適用

### 採用する設計パターン

| パターン | 適用 | 備考 |
|---|---|---|
| 3層分離（Platform/Surface/Core） | sdit / sdit-session / sdit-core | libghostty 思想の Rust 版 |
| Mailbox イベントキュー | App ↔ Session 間通信 | ロックフリーキュー |
| DerivedConfig（設定コピー） | Session 毎の設定 | hot reload 対応 |
| Dirty Flags | Terminal → Renderer | 差分描画の判定 |
| ScreenSet (Primary/Alternate) | sdit-core Terminal | VT100 仕様 |
| SharedGridSet（フォント共有） | App レベル | メモリ効率化 |

### スレッド分離モデル（Ghostty に倣う）

```
Main Thread (App loop)
  ├─ Mailbox ドレイン
  ├─ Session 追加・削除
  └─ キーバインド転送

Session Thread (io_thread 相当) ← Session 毎に1つ
  ├─ PTY 読み込み → VTE パース
  ├─ Terminal 状態書き込み
  └─ "dirty" イベント送信

Render Thread (renderer_thread 相当)
  ├─ Terminal 状態を RwLock で読み取り
  ├─ GPU 描画
  └─ "draw" リクエスト送信
```

### SDIT 固有の拡張

Ghostty は「1 Surface = 1 Terminal」だが、SDIT では Session と表示先を分離:

```
Session（世界に一つ、PTY 実体）
  ├─ Screen state
  ├─ Color/Mode state
  └─ Dirty flags

  ↑ displayed in ↑

Surface A (Window X)
Surface B (Tab in Window Y)
```

合体・切出し時は Surface を差し替えるだけ。Session（PTY）は切れない。

### 採用しないもの

- Zig コード自体（言語が違う）
- macOS AppKit 統合コード（Ghostty 固有）
- Zig の C インターフェース生成部分
