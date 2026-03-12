# アーキテクチャ判断メモ（Phase 0 読解から得た知見）

## 1. 2クレート構成（Phase 5.8 で統合）

Phase 5.8 で sdit-config/sdit-session/sdit-render を sdit-core に統合。
過分割（5クレート・平均1350行）を解消し、2クレート構成に簡素化。

```
sdit-core (lib)
  ├─ terminal/  — VTE ステートマシン
  ├─ grid/      — セルグリッド・スクロールバック
  ├─ pty/       — PTY プロセス管理
  ├─ font/      — 低レベルフォント処理
  ├─ render/    — wgpu パイプライン・アトラス・フォントコンテキスト
  ├─ session/   — セッション管理・サイドバー状態・永続化
  └─ config/    — TOML 設定・カラーテーマ

sdit (bin, GUI 依存)
  ├─ winit イベントループ
  ├─ GPU コンテキスト初期化
  ├─ SessionSidebar（縦タブ UI）
  └─ Mailbox（スレッド間通信）
```

再分割の判断基準: 単一モジュールが1500行超、独立コンパイルサイクル必要、外部公開理由あり。

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
