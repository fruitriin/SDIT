# WezTerm: Mux レイヤー・SDI変換 読解メモ

## 対象ファイル
- `refs/wezterm/mux/src/` — セッション多重化レイヤー
- `refs/wezterm/wezterm-gui/src/termwindow/mod.rs` — ウィンドウ実装

---

## 1. Mux レイヤー設計

### Mux 構造体の管理状態

```rust
Mux {
    tabs: HashMap<TabId, Arc<Tab>>,
    panes: HashMap<PaneId, Arc<dyn Pane>>,
    windows: HashMap<WindowId, Window>,
    domains: HashMap<DomainId, Arc<dyn Domain>>,
    default_domain: Option<Arc<dyn Domain>>,
    subscribers: HashMap<usize, Box<dyn Fn(MuxNotification) -> bool>>,
    clients: HashMap<ClientId, ClientInfo>,
    num_panes_by_workspace: HashMap<String, usize>,
}
```

### オブジェクト階層

```
Window
  ├─ Tab（複数、順序付きベクタ）
  │   └─ Pane（ツリー構造で分割可能）
  │        └─ Domain（Pane が属する実行環境）
  └─ workspace: String
```

- **Pane**: PTY 入出力の抽象化。キーボード入力を受け取り画面状態を管理
- **Tab**: Pane をツリー構造で保持。ペイン分割を管理
- **Window**: 複数の Tab を保持。アクティブタブを追跡
- **Domain**: Pane のファクトリ。LocalDomain, ClientDomain, RemoteSshDomain 等

### セッション ≠ ウィンドウ の分離

Pane ≈ セッション（PTY ライフサイクル）として機能し、ウィンドウと独立して存在:

```
[Window A] ←表示→ Pane_1
[Window B] ←表示→ Pane_2
         ↓ ドラッグで合体
[Window A: Tab_1(Pane_1) | Tab_2(Pane_2)]
（Pane の PTY 接続は不変）
```

### セッション生成フロー

```
Domain::spawn(window_id)
  ├─ spawn_pane() で Arc<dyn Pane> 作成
  ├─ Tab::new() で新規 Tab
  ├─ tab.assign_pane(&pane)
  ├─ mux.add_tab_and_active_pane(&tab) で Mux に登録
  └─ mux.add_tab_to_window(&tab, window_id) で Window に追加
```

### 破棄フロー

- Pane 終了 → `MuxNotification::PaneRemoved`
- Tab がペインレス → 自動削除
- Window がタブレス → `MuxNotification::WindowRemoved`

---

## 2. ウィンドウライフサイクル

### TermWindow 構造体

TermWindow = ウィンドウ 1 枚の UI レイヤー:

- `window: Option<Window>` — ネイティブウィンドウ
- `mux_window_id: MuxWindowId` — Mux::Window との対応
- `render_state: Option<RenderState>` — 描画状態
- `tab_bar: TabBarState` — タブバー描画状態
- `pane_state: HashMap<PaneId, PaneState>` — ペイン毎の UI 状態

### ウィンドウと Mux の接続 — Notification パターン

```
Mux
  ├─ notify(MuxNotification)
  │   ├─ WindowCreated(window_id)
  │   ├─ TabAddedToWindow { tab_id, window_id }
  │   ├─ PaneOutput(pane_id)
  │   ├─ WindowInvalidated(window_id)
  │   └─ WindowRemoved(window_id)
  │
  └─ subscribers
      └─ TermWindow が購読 → TermWindowNotif へ変換
```

購読は `mux.subscribe(move |n| { window.notify(...) })` で実装。

---

## 3. SDI 変換の設計

### WezTerm → SDIT 概念マッピング

| WezTerm | SDIT | 差異 |
|---|---|---|
| Pane | Session | ペイン分割なし（1:1 単純化） |
| Tab | SessionRef | 1 Tab = 1 Session（ツリー構造不要） |
| Window | Window | タブ数 1 でタブバー非表示 |
| Domain | SessionFactory | ローカル PTY 生成のみ（初期） |
| Workspace | — | 将来検討 |

### SDIT に適用する WezTerm パターン

**1. Domain ファクトリパターン**
- Session 生成を Domain に一元化
- 将来の SSH リモートセッション等に拡張可能

**2. Notification システム**
- `SessionAdded`, `SessionRemoved` 等のイベントで UI 動的更新
- 縦タブバーの自動表示・非表示に活用

**3. Arc<dyn Pane> による共有所有権**
- Session を `Arc<Session>` で管理
- Window 間の移動時に所有権移転ではなく参照の付け替え

**4. Window のセッション数に応じた UI 切り替え**
```rust
if window.session_count() > 1 {
    show_vertical_tabbar();
} else {
    hide_tabbar(); // SDI 状態
}
```

### Chrome-like UX の実現

**タブ合体:**
1. ドラッグ中の Session 参照を保持
2. ドロップ先 Window ID を決定
3. `window.add_session(session)` — Session の PTY は不変

**タブ切出し:**
1. `window.remove_session(session_id)` で Session を取り外し
2. 新 Window 生成
3. `new_window.add_session(session)` で追加

**重要**: Session のライフサイクルは Window/Tab 操作に影響されない。

---

## SDITへの適用

### 採用する設計

| 要素 | 適用先 | 備考 |
|---|---|---|
| Mux 一元管理 | SessionManager | 全 Session/Window を管理 |
| Domain ファクトリ | SessionFactory | Session 生成の抽象化 |
| MuxNotification | SessionEvent | イベント駆動 UI 更新 |
| Arc<dyn Pane> 共有 | Arc<Session> | Window 間移動に必須 |
| Window の Tab 管理 | Window::sessions | Vec で順序管理 |

### 単純化するもの

| WezTerm の複雑性 | SDIT での対応 |
|---|---|
| Pane ツリー構造（bintree） | 不要（ペイン分割しない） |
| 複数 Domain 同時管理 | LocalDomain のみ（初期） |
| Lua 設定エンジン | TOML で十分 |
| Tab ↔ Pane の N:M 関係 | Session = Tab = Pane（1:1:1） |

### 採用しないもの

- 水平タブバーの実装（設計が逆）
- Lua 設定エンジン
- wgpu 以外のレンダーバックエンド
