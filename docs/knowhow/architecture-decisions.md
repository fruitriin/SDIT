# アーキテクチャ判断メモ（Phase 0 読解から得た知見）

## 1. 3層分離アーキテクチャ（Ghostty libghostty 参照）

Rust での実現方針:

```
sdit-core (lib, GUI ゼロ依存)
  ├─ Terminal（VTE ステートマシン）
  ├─ Grid/Screen（セルデータ + リングバッファ）
  ├─ PTY I/O 抽象（trait PtyBackend）
  └─ Dirty Flags（差分描画判定）

sdit-session (lib, GUI ゼロ依存)
  ├─ SessionManager（全 Session/Window 管理、Mux 相当）
  ├─ Session = PTY + Terminal（ライフサイクル独立）
  ├─ WindowRegistry（SDI ウィンドウ一覧）
  └─ SessionEvent（Notification 型イベント）

sdit (bin, GUI 依存)
  ├─ winit イベントループ
  ├─ wgpu レンダラー
  ├─ SessionSidebar（縦タブ UI）
  └─ Mailbox（スレッド間通信）
```

## 2. スレッドモデル

- **Main Thread**: App ループ、Mailbox ドレイン、ウィンドウ管理
- **Session Thread**: Session 毎に 1 つ。PTY 読み書き → VTE パース
- **Render Thread**: Terminal 状態を RwLock で読み取り → GPU 描画
- スレッド間通信は Mailbox パターン（ロックフリーキュー）

## 3. グリッド実装方針（Alacritty 参照）

- Row<Cell> + Storage<T> リングバッファ
- `zero` オフセットによる O(1) スクロール
- `display_offset` でスクロールバック位置管理
- Damage tracking で差分描画

## 4. Session ≠ Window 分離（WezTerm 参照）

- Session は Arc で管理、Window 間で移動可能
- 合体・切出し時は Surface（表示先）を差し替えるだけ
- PTY 接続は Window 操作に影響されない

## 5. 縦タブバー（Zellij 参照）

- レイアウト計算結果をキャッシュ（毎フレーム再計算しない）
- Y 座標で線形探索してクリック対象を特定
- スペース不足時は折り畳み表示（「↑ +N」「+N ↓」）

## 6. 要再調査事項

- `polling` vs `tokio` — PTY I/O の非同期化方針。Alacritty は polling だが、複数 Session 管理時に tokio が有利な可能性
- `cosmic-text` vs 独自フォントシェーピング — Ghostty のフォント処理は Zig 固有のため直接参照不可
- スクロールバックの `MAX_CACHE_SIZE` 最適値 — メモリ使用量とのトレードオフ
