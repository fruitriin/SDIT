# Zellij: タブバー・セッション状態型 読解メモ

## 対象ファイル
- `refs/zellij/default-plugins/tab-bar/src/` — タブバープラグイン
- `refs/zellij/zellij-utils/src/data.rs` — セッション状態型

---

## 1. タブバープラグイン

### 基本構造

**LinePart（描画の最小単位）:**
```rust
LinePart {
    part: String,              // ANSI 着色済み文字列
    len: usize,               // 表示幅（Unicode サポート）
    tab_index: Option<usize>, // クリック時のタブ識別用
}
```

**State（プラグイン状態）:**
```rust
State {
    tabs: Vec<TabInfo>,           // Zellij から受信
    active_tab_idx: usize,        // アクティブタブ（1-indexed）
    mode_info: ModeInfo,          // キーバインド・スタイル・モード
    tab_line: Vec<LinePart>,      // レンダリングキャッシュ
}
```

### イベント処理

3種類の更新イベント:

1. **TabUpdate**: タブリスト変更 → 差分検出で再描画
2. **ModeUpdate**: スタイル・カラースキーム変更 → 再描画
3. **Mouse**: クリック → `get_tab_to_focus()` で該当タブ判定 → 切り替え

### マウスクリック処理

```rust
fn get_tab_to_focus(tab_line: &[LinePart], active_tab_idx: usize, col: usize) -> Option<usize> {
    let mut len = 0;
    for part in tab_line {
        if col >= len && col < len + part.len {
            return Some(part.tab_index?);
        }
        len += part.len;
    }
    None
}
```

累計幅で線形探索し、該当タブを特定。

### スクロールイベント

- ScrollUp → 次のタブ
- ScrollDown → 前のタブ

### タブの状態表示

- `is_fullscreen_active` → "(FULLSCREEN)"
- `is_sync_panes_active` → "(SYNC)"
- `has_bell_notification` / `is_flashing_bell` → "[!]"
- `other_focused_clients` → マルチプレイヤー表示

### カラリング

- `ribbon_selected` — アクティブタブ
- `ribbon_unselected.background` — 非アクティブ（奇数）
- `ribbon_unselected.emphasis_1` — 非アクティブ（偶数）
- ベル通知時は `emphasis_3` で強調

### スペース制約下のレイアウト

アクティブタブを中心に、左右交互に優先度を付けてタブを配置:
- 入りきらないタブは「← +N」「+N →」で折り畳み表示
- プレフィックス（セッション名）とサフィックス（レイアウト名）を前後に配置

---

## 2. セッション状態型

### TabInfo

```rust
TabInfo {
    position: usize,                      // 0-indexed 位置
    name: String,                        // タブ名（UI 表示）
    active: bool,                        // アクティブ状態
    is_fullscreen_active: bool,
    is_sync_panes_active: bool,
    other_focused_clients: Vec<ClientId>,
    active_swap_layout_name: Option<String>,
    is_swap_layout_dirty: bool,
    viewport_rows/columns: usize,
    tab_id: usize,                       // 安定識別子
    has_bell_notification: bool,
    is_flashing_bell: bool,
}
```

### SessionInfo

```rust
SessionInfo {
    name: String,                        // セッション名
    tabs: Vec<TabInfo>,                  // タブリスト
    panes: PaneManifest,                 // ペイン情報
    connected_clients: usize,
    is_current_session: bool,
    available_layouts: Vec<LayoutInfo>,
    tab_history: BTreeMap<ClientId, Vec<usize>>,
    creation_time: Duration,
}
```

---

## SDITへの適用

### 縦タブバーの設計

**表示・非表示の切り替え:**
- セッション数 ≤ 1: タブバー非表示（SDI 状態）
- セッション数 ≥ 2: 縦タブバー自動出現
- `Cmd+\` で手動トグル

**レイアウト構造:**
```
┌──────────┐
│ [上部]   │  ← ウィンドウ情報（オプション）
├──────────┤
│ Session1 │  ← スクロール可能なリスト
│ Session2 │
│ Session3 │
├──────────┤
│ [下部]   │  ← コマンド（オプション）
└──────────┘
```

**スペース制約対策:**
- ウィンドウ高さ不足時「↑ +N」「+N ↓」で折り畳み
- アクティブセッションを優先表示（Zellij のアクティブタブ中心配置を参考）

### SDIT のセッション型設計（Zellij 参考）

Zellij の SessionInfo → TabInfo → Pane の3階層を単純化:

```rust
// SDIT 版
struct Session {
    id: SessionId,           // 安定識別子
    name: String,           // ユーザー変更可
    active: bool,
    has_bell: bool,
    is_flashing_bell: bool,
    created_at: SystemTime,
}
```

### 縦タブの操作

| 操作 | 実装 |
|---|---|
| クリック → セッション切替 | Y 座標で線形探索（Zellij の X 座標版を転用） |
| スクロール → 上下選択 | ScrollUp/Down イベント |
| ドラッグ → ウィンドウ間移動 | Chrome-like UX（Zellij には未実装） |
| 右クリック → メニュー | リネーム・削除（将来実装） |

### キャッシュ設計

Zellij の `tab_line: Vec<LinePart>` キャッシュに倣い、縦タブバーもレイアウト計算結果をキャッシュ:
- タブリスト変更時のみ再計算
- 毎フレーム再計算を避ける

### 採用しないもの

- ペイン分割システム全体
- WebAssembly プラグインシステム
- TUI レンダリング層（SDIT はネイティブ GPU 描画）
- 水平方向のタブバー配置
