# 複数ウィンドウ + Session 管理の知見

## Session ≠ Window 分離パターン

Session（PTY + Terminal状態）と Window（GUI）を完全に分離する設計。

```
Session (sdit-session)        Window (sdit バイナリ)
├── SessionId                 ├── WindowId (winit)
├── TerminalState             ├── GpuContext
│   ├── Terminal              ├── CellPipeline
│   └── Processor             ├── Atlas
├── PtyIo                     └── SessionId (参照)
│   ├── write_tx
│   ├── _reader thread
│   └── _writer thread
```

- Session は GUI に依存しない（`sdit-session` は winit を知らない）
- Window は SessionId で間接的に Session を参照
- `SessionManager` がセッションの CRUD と ID 採番を担当

## spawn_reader クロージャパターン

`sdit-session` が winit に依存しないよう、PTY Reader/Writer スレッドの生成を呼び出し側のクロージャに委譲する:

```rust
Session::spawn(id, SpawnParams {
    spawn_reader: |pty, term_state| {
        // ここで EventLoopProxy を使える（sdit バイナリ側のスコープ）
        let reader = spawn_pty_reader(pty, term_state, event_proxy, sid);
        let writer = spawn_pty_writer(writer, rx, event_proxy, sid);
        (reader, writer, tx)
    },
    ...
})
```

これにより、sdit-session のテストでは EventLoop なしで Session を生成でき、
sdit バイナリでは winit の EventLoopProxy を注入できる。

## SditEvent に SessionId を含める

複数セッション対応では、PTY イベントにどのセッションからのものかを明示する:

```rust
enum SditEvent {
    PtyOutput(SessionId),      // どのセッションの出力か
    ChildExit(SessionId, i32), // どのセッションが終了したか
}
```

`SessionId → WindowId` の逆引きマップで正しいウィンドウを再描画する。

## ウィンドウ閉じのライフサイクル

1. `CloseRequested` → `close_window(id)` → Session 削除 → PTY/スレッドは detach
2. `ChildExit(sid)` → 逆引きで WindowId 取得 → `close_window(wid)` → Session 削除
3. `windows.is_empty()` → `event_loop.exit()`

## 既知の制限（Phase 3.1 で対応予定）

- Session drop 時にスレッドが join されない（detach のまま）
- PTY resize が子プロセスに伝播しない（SIGWINCH 未送信）
