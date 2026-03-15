# Ghostty: macOS ネイティブタブ実装 読解メモ

> 対象コミット: refs/ghostty サブモジュール参照先  
> 読解日: 2025年

---

## 全体アーキテクチャ

### TerminalController / BaseTerminalController の役割分担

Ghostty の macOS 実装は、**基底クラスが意図的にタブを扱わない**設計になっている。

```
BaseTerminalController  ←  タブを実装しない (ドキュメントに明示)
   ↑ 継承
TerminalController      ←  タブ・ウィンドウ管理を担当
```

`BaseTerminalController` のクラスコメントには以下の記述がある:

```swift
/// Notably, things this class does NOT implement (not exhaustive):
///
///   - Tabbing, because there are many ways to get tabbed behavior in macOS and we
///   don't want to be opinionated about it.
///   - Window restoration or save state
///   - Window visual styles (such as titlebar colors)
```

`BaseTerminalController` が担う機能:
- `surfaceTree: SplitTree<Ghostty.SurfaceView>` の管理（分割画面）
- フォーカスされた Surface の追跡 (`focusedSurface`)
- NSWindowDelegate の基本実装（`windowShouldClose`, `windowWillClose` 等）
- クリップボード確認ダイアログ
- フルスクリーン制御
- タイトル管理 (`titleOverride`, `lastComputedTitle`)
- ベル通知の集約

`TerminalController` が追加する機能:
- タブの作成・切り替え・移動・閉じる操作
- 複数ウィンドウのカスケード配置
- Undo/Redo 管理（タブ・ウィンドウ単位）
- タブ色・タブラベル
- タイトルバースタイル別の NIB ファイル選択

この分離により、例えば `QuickTerminalController` のような「タブを持たない特殊なターミナルウィンドウ」を実装できる設計になっている。

### NSWindow のタブ機能をどう使っているか

macOS の `NSWindow` は標準でタブグループ（`NSWindowTabGroup`）の概念を持つ。Ghostty はこれを直接使っている。

**基本的な仕組み:**
- 1タブ = 1 `NSWindow` = 1 `TerminalController`
- 複数のウィンドウが `NSWindowTabGroup` に束ねられることでタブとして表示される
- タブバーの表示・非表示は macOS が自動で制御する（グループに2個以上のウィンドウがある場合に表示）

**重要な設計判断 — macOS の自動タブ挿入を無効化している:**

`windowDidLoad` に以下のコードがある:

```swift
// In various situations, macOS automatically tabs new windows. Ghostty handles
// its own tabbing so we DONT want this behavior.
if !window.styleMask.contains(.fullScreen) {
    if let tabGroup = window.tabGroup, tabGroup.windows.count > 1 {
        window.tabGroup?.removeWindow(window)
    }
}
```

macOS には「システム設定 → デスクトップとDock → 書類をタブで開く = 常に」という設定があり、これが有効だと新しいウィンドウが自動的に既存のタブグループに追加される。Ghostty はこれを検知してすぐに取り除き、自分でタブの配置を制御する。

### Surface（Zig コア）と Swift GUI レイヤーの接続方法

Ghostty のアーキテクチャは Zig コアと Swift GUI の2層構造:

```
Surface.zig (Zig コア)
  PTY + ターミナルステートマシン
  キーバインド処理
  タブアクションを rt_app.performAction() で上位に委譲
        ↓ C FFI (GhosttyKit)
Ghostty.Surface.swift (Swift ラッパー)
  ghostty_surface_t の Swift ラッパークラス
        ↓ NotificationCenter
AppDelegate.swift
  ghosttyNewTab / ghosttyNewWindow 通知を受信
        ↓
TerminalController.newTab() / newWindow()
```

`Surface.zig` の冒頭コメントが設計思想を説明している:

```zig
//! Surface represents a single terminal "surface". A terminal surface is
//! a minimal "widget" where the terminal is drawn and responds to events
//! such as keyboard and mouse. Each surface also creates and owns its pty
//! session.
//!
//! The word "surface" is used because it is left to the higher level
//! application runtime to determine if the surface is a window, a tab,
//! a split, a preview pane in a larger window, etc. This struct doesn't care:
//! it just draws and responds to events.
```

つまり Zig コアは「自分がタブなのかウィンドウなのか」を知らない。タブ操作のアクションが発火されると `rt_app.performAction()` で上位（Swift）に委譲するだけ:

```zig
.new_tab => return try self.rt_app.performAction(
    .{ .surface = self },
    .new_tab,
    {},
),

.close_tab => |v| return try self.rt_app.performAction(
    .{ .surface = self },
    .close_tab,
    switch (v) {
        .this => .this,
        .other => .other,
        .right => .right,
    },
),

inline .previous_tab, .next_tab, .last_tab, .goto_tab,
=> |v, tag| return try self.rt_app.performAction(
    .{ .surface = self },
    .goto_tab,
    switch (tag) {
        .previous_tab => .previous,
        .next_tab => .next,
        .last_tab => .last,
        .goto_tab => @enumFromInt(v),
        else => comptime unreachable,
    },
),

.move_tab => |position| return try self.rt_app.performAction(
    .{ .surface = self },
    .move_tab,
    .{ .amount = position },
),
```

---

## タブのライフサイクル

### 新規タブ作成の流れ

**Zig コア → Swift の流れ:**
1. ユーザーがキーバインドを押す（例: `new_tab`）
2. `Surface.zig` の `performBindingAction(.new_tab)` が実行される
3. `rt_app.performAction(.new_tab, {})` で Swift に通知
4. `AppDelegate.ghosttyNewTab(_:)` が受信:

```swift
@objc private func ghosttyNewTab(_ notification: Notification) {
    guard let surfaceView = notification.object as? Ghostty.SurfaceView else { return }
    guard let window = surfaceView.window else { return }
    guard window.windowController is TerminalController else { return }
    let config = ...
    _ = TerminalController.newTab(ghostty, from: window, withBaseConfig: config)
}
```

**`TerminalController.newTab()` の詳細:**

```swift
static func newTab(_ ghostty: ..., from parent: NSWindow?, ...) -> TerminalController? {
    // 1. 新しいウィンドウ（= 新しいPTY）を作成
    let controller = TerminalController.init(ghostty, withBaseConfig: baseConfig)
    guard let window = controller.window else { return controller }

    // 2. 親が最小化されていたら復元
    if parent.isMiniaturized { parent.deminiaturize(self) }

    // 3. macOS が勝手に追加した場合は先に除去（タブバーの「+」ボタン対策）
    if let tg = parent.tabGroup, tg.windows.firstIndex(of: window) != nil {
        tg.removeWindow(window)
    }

    // 4. タブとして追加
    switch ghostty.config.windowNewTabPosition {
    case "end":
        if let last = parent.tabGroup?.windows.last {
            last.addTabbedWindowSafely(window, ordered: .above)
        } else {
            parent.addTabbedWindowSafely(window, ordered: .above)
        }
    case "current": fallthrough
    default:
        parent.addTabbedWindowSafely(window, ordered: .above)
    }

    // 5. メインループの次ティックで表示 (カスケード処理の都合)
    DispatchQueue.main.async {
        controller.showWindow(self)
        window.makeKeyAndOrderFront(self)
    }

    // 6. 0.1秒後にタブラベルを更新
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
        controller.relabelTabs()
    }
}
```

**0.1秒の遅延が必要な理由:**
コードコメントによると、タブバーの「+」ボタンをクリックした場合、macOS のタブグループ状態が次のイベントループティックまで安定しないため、ラベリングがずれてしまう。これを避けるための遅延。

### タブの切り出し（ドラッグでウィンドウに分離）の処理

Ghostty は macOS のネイティブドラッグ操作に任せている。macOS はタブをドラッグアウトすると自動的に `NSWindowTabGroup` から取り除き、独立した `NSWindow` にする。Swift 側では特別な処理は不要。

ただし、**分割画面（Split）のドラッグ切り出し**は別途実装されている (`ghosttySurfaceDragEndedNoTarget`):

```swift
@objc private func ghosttySurfaceDragEndedNoTarget(_ notification: Notification) {
    // ドラッグされた Surface を現在のツリーから除去
    let removedTree = surfaceTree.removing(targetNode)
    
    // 新しいウィンドウをドロップ位置に作成
    let newTree = SplitTree<Ghostty.SurfaceView>(view: target)
    replaceSurfaceTree(removedTree, ...)
    _ = TerminalController.newWindow(ghostty, tree: newTree, position: dropPosition)
}
```

### タブの結合（ウィンドウをタブにマージ）の処理

macOS の標準 "Window → Merge All Windows" メニューが使える（`mergeAllWindows(_:)`）。Ghostty はこれを特別に実装していない——macOS の AppKit が自動的に全ウィンドウを1つのタブグループにまとめる。

Undo によるタブ復元時は明示的に `addTabbedWindowSafely` を呼ぶ:

```swift
// Undo: タブグループを復元する場合
for controller in controllers.dropFirst() {
    controller.showWindow(nil)
    if let firstWindow = firstController.window,
       let newWindow = controller.window {
        firstWindow.addTabbedWindowSafely(newWindow, ordered: .above)
    }
}
```

### タブを閉じる処理

```
closeTab(sender) 
  → タブが1つしかない場合は closeWindow へ
  → 実行中プロセスがあれば確認ダイアログ
  → closeTabImmediately()
      → tabGroup.windows.count > 1 でなければ closeWindowImmediately() へ
      → window.close()  ← NSWindow を閉じるだけ。PTY は TerminalController の deinit で解放
```

**重要:** `window.close()` はタブグループから NSWindow を取り除くが、`TerminalController` の `deinit` が呼ばれるまで PTY は生き続ける。これにより Undo のための状態保持が可能になっている。

---

## NSWindow タブ API の使い方

### `window.tabbedWindows`

タブグループ内の全 NSWindow の配列。順序はタブの表示順序に対応する。

```swift
// タブ数でラベル更新の要否を判断
tabListenForFrame = window?.tabbedWindows?.count ?? 0 > 1

// タブに順番にキーショートカットを割り当て
if let windows = window?.tabbedWindows as? [TerminalWindow] {
    for (tab, window) in zip(1..., windows) {
        guard tab <= 9 else {
            window.keyEquivalent = ""
            continue
        }
        if let equiv = ghostty.config.keyboardShortcut(for: "goto_tab:\(tab)") {
            window.keyEquivalent = "\(equiv)"
        }
    }
}
```

### `window.addTabbedWindow(_:ordered:)`

別のウィンドウをタブグループに追加する。Ghostty は AppKit が例外を投げることがあるため、Objective-C ラッパーを介して安全に呼び出す:

```swift
// NSWindow+Extension.swift
@discardableResult
func addTabbedWindowSafely(_ child: NSWindow, ordered: NSWindow.OrderingMode) -> Bool {
    var error: NSError?
    let success = GhosttyAddTabbedWindowSafely(self, child, ordered.rawValue, &error)
    if let error {
        Ghostty.logger.error("addTabbedWindow failed: \(error.localizedDescription)")
    }
    return success
}
```

`ordered: .above` は呼び出し元の右隣に挿入、`ordered: .below` は左隣に挿入する。

### `window.moveTabToNewWindow(_:)` / `window.mergeAllWindows(_:)`

これらの macOS 標準アクションは Ghostty では直接呼ばない。ユーザーが MainMenu から操作した場合は AppKit が内部で処理する。

### `NSWindowDelegate` のタブ関連メソッド

`windowWillClose` でタブラベルを更新:

```swift
override func windowWillClose(_ notification: Notification) {
    super.windowWillClose(notification)
    self.relabelTabs()
    // カスケードポイントのリセット処理...
}
```

`windowDidBecomeKey` でタブラベルを更新:

```swift
override func windowDidBecomeKey(_ notification: Notification) {
    super.windowDidBecomeKey(notification)
    self.relabelTabs()
    self.fixTabBar()
}
```

**タブ再配置のイベント検知（ハック）:**

macOS はタブをマウスでドラッグして並び替えるとき、専用のデリゲートメソッドや通知を提供しない。そのため Ghostty は以下のハックを使っている:

```swift
// tabListenForFrame = true のとき（タブが2個以上のとき）有効
@objc private func onFrameDidChange(_ notification: NSNotification) {
    guard tabListenForFrame else { return }
    // tabbedWindows 配列のハッシュが変わった = 順序変更が起きた
    guard let v = self.window?.tabbedWindows?.hashValue else { return }
    guard tabWindowsHash != v else { return }
    tabWindowsHash = v
    self.relabelTabs()
}
```

タブのアクセサリービューに `postsFrameChangedNotification = true` を設定し、フレーム変更通知を受信する。`tabbedWindows` 配列のハッシュ値が変わったらタブの順序が変わったと判断する。

### タブバーの自動表示/非表示

macOS の AppKit がタブグループ内のウィンドウ数に応じて自動で制御する。コードでは制御していない。ただし透明ウィンドウの場合、タブバーが「遅延」して背景を引きずって描画される問題があるため、`fixTabBar()` が各種コールバックで呼ばれる:

```swift
private func fixTabBar() {
    // 透明ウィンドウでタブバーが背景を引きずる問題の回避策
    // isOpaque を true → false にトグルして再描画を強制する
    if let window = window, !window.isOpaque {
        window.isOpaque = true
        window.isOpaque = false
    }
}
```

---

## Surface / セッション管理

### 1ウィンドウ = 1 Surface の関係がタブ時にどうなるか

Ghostty のタブでは **1タブ = 1ウィンドウ = 1TerminalController = 1SurfaceTree** の関係が維持される。タブが合体しても、内部的に各タブは独立した NSWindow として存在し続ける。

```
タブグループ (NSWindowTabGroup)
├── NSWindow A → TerminalController A → SurfaceTree A → PTY A
├── NSWindow B → TerminalController B → SurfaceTree B → PTY B
└── NSWindow C → TerminalController C → SurfaceTree C → PTY C
```

表示は macOS の NSWindowTabGroup がまとめて1つのウィンドウ枠に収めているが、内部は独立したウィンドウのままである。

### タブ切り出し時に PTY / Surface はどう維持されるか

タブの切り出し（ドラッグアウト）は NSWindow をタブグループから取り除くだけ。`TerminalController` は変更されないため、PTY も `surfaceTree` も影響を受けない。

```
切り出し前:
  タブグループ [Win A (PTY A), Win B (PTY B)]
  
切り出し後:
  独立ウィンドウ Win A (PTY A)    ← PTY Aはそのまま生きている
  独立ウィンドウ Win B (PTY B)    ← PTY Bもそのまま生きている
```

**これが Ghostty の「シンプルさ」の核心**。Ghostty の Surface はタブであることを知らない (`Surface.zig` コメント参照)。窓の外観が変わるだけで、内側のPTYには触れない。

### Surface のライフサイクル（作成→タブ移動→破棄）

```
1. 作成:
   TerminalController.init() 
   → BaseTerminalController.init()
   → SurfaceView(ghostty_app, baseConfig: base) // PTY + Zigコアのsurfaceを生成
   → surfaceTree = .init(view: surfaceView)

2. タブとして追加:
   parent.addTabbedWindowSafely(window, ordered: .above)
   // NSWindowの親子関係が変わるだけ、SurfaceViewは不変

3. タブ移動（並び替え）:
   tabGroup.removeWindow(selectedWindow)
   targetWindow.addTabbedWindowSafely(selectedWindow, ordered: ...)
   // NSWindowレベルの操作のみ、PTYは不変

4. 破棄:
   window.close()
   → windowWillClose
   → window.contentView = nil  // SwiftUI ビュー階層を解放
   → TerminalController deinit → surfaceTree が解放される
   → SurfaceView deinit → ghostty_surface_free(surface) // PTYを終了
```

`Ghostty.Surface.swift` の deinit は非同期で実行される（メインアクタで実行する必要があるが deinit はどこで呼ばれるか保証がないため）:

```swift
deinit {
    let surface = self.surface
    Task.detached { @MainActor in
        ghostty_surface_free(surface)
    }
}
```

---

## タブタイトル

### TabTitleEditor の仕組み

タブバー上でタブタイトルをダブルクリックするとインライン編集ができる。この機能は `TabTitleEditor` クラスが担当する。

**動作フロー:**
1. `TerminalWindow`（`NSWindow` サブクラス）がマウスイベントを受信
2. `handleMouseDown(_:)` でダブルクリックを検知
3. `hostWindow.tabIndex(atScreenPoint:)` で何番目のタブがクリックされたか取得
4. 0.1秒後（次のイベントループティック）に `beginEditing(for:)` を呼ぶ

**プライベート API 使用:**

```swift
// NSWindow+Extension.swift のプライベートAPI
var tabBarView: NSView? {
    titlebarView?.firstDescendant(withClassName: "NSTabBar")
}

func tabButtonsInVisualOrder() -> [NSView] {
    guard let tabBarView else { return [] }
    return tabBarView
        .descendants(withClassName: "NSTabButton")
        .sorted { $0.frame.minX < $1.frame.minX }
}
```

`NSTabBar` と `NSTabButton` は macOS の非公開クラス。クラス名でビュー階層を辿る方法は将来のOS更新で壊れる可能性がある。

**インライン編集の実装:**
- タブボタン内のラベル（`NSTextField`）を一時的に隠す
- 同じ位置に編集用の `NSTextField` を挿入
- Enter で確定（`finishEditing(commit: true)`）
- Escape でキャンセル（`finishEditing(commit: false)`）

**タイトルの優先順位:**

```swift
// BaseTerminalController
private func applyTitleToWindow() {
    if let titleOverride {
        // ユーザーが手動で設定したタイトルが最優先
        window.title = computeTitle(title: titleOverride, bell: ...)
        return
    }
    // デフォルト: PTYのターミナルタイトル（OSCシーケンスで設定される）
    window.title = lastComputedTitle
}
```

`titleOverride` は `BaseTerminalController` の `promptTabTitle()` や `TabTitleEditor` のコミットコールバックで設定される。空文字列で設定すると nil にリセットされてデフォルトに戻る。

### タブ色（TerminalTabColor）

`TerminalTabColor` は 10色 + None の列挙型:

```swift
enum TerminalTabColor: Int, CaseIterable, Codable {
    case none, blue, purple, pink, red, orange, yellow, green, teal, graphite
}
```

各色に `displayColor: NSColor?` が対応する（例: `.blue` → `.systemBlue`）。これは `TerminalWindow` の `tabColor` プロパティとして設定される。Undo/Redo のためにタブ色は `UndoState` に保存される:

```swift
struct UndoState {
    let frame: NSRect
    let surfaceTree: SplitTree<Ghostty.SurfaceView>
    let focusedSurface: UUID?
    let tabIndex: Int?
    weak var tabGroup: NSWindowTabGroup?
    let tabColor: TerminalTabColor  // ← Undo/Redoで復元される
}
```

---

## タブグループ

### TabGroupCloseCoordinator — タブグループ全体を閉じるときの調整

**問題:** macOS のネイティブタブで「ウィンドウを閉じる」（赤×ボタン）を押すと、タブグループ内の全ウィンドウそれぞれに `windowShouldClose(_:)` が連続して呼ばれる。「タブを1つ閉じたい」のか「ウィンドウ全体（全タブ）を閉じたい」のかを区別する仕組みが必要。

**解決策（100ms デバウンスタイマー）:**

```swift
func windowShouldClose(_ window: NSWindow, callback: @escaping Callback) {
    // タブグループ全ウィンドウから close リクエストが来たら "window" と判断
    if closeRequests.count == tabGroup.windows.count {
        let allWindows = Set(tabGroup.windows.map { ObjectIdentifier($0) })
        if Set(closeRequests.keys) == allWindows {
            trigger(.window)
            return
        }
    }
    
    // 100ms 以内に全タブからリクエストが来なければ "tab" と判断
    debounceTimer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: false) { _ in
        self?.trigger(.tab)
    }
}
```

**コーディネータの共有:** タブグループの最初のウィンドウのコーディネータが代表になる:

```swift
if let firstController = tabGroup.windows.first?.windowController as? Controller,
   firstController.tabGroupCloseCoordinator !== self {
    let coordinator = firstController.tabGroupCloseCoordinator
    coordinator.windowShouldClose(window, callback: callback)
    return
}
```

`TerminalController.windowShouldClose` はこのコーディネータを使って判断を待ち、結果に応じて `closeTab(nil)` または `closeWindow(nil)` を呼ぶ:

```swift
override func windowShouldClose(_ sender: NSWindow) -> Bool {
    tabGroupCloseCoordinator.windowShouldClose(sender) { [weak self] scope in
        switch scope {
        case .tab: closeTab(nil)
        case .window:
            guard self.window?.isFirstWindowInTabGroup ?? false else { return }
            closeWindow(nil)
        }
    }
    return false  // 常に false を返す（明示的に閉じる）
}
```

---

## AppDelegate のタブ関連処理

### newTab / newWindow の dispatch

**通知ベースの間接呼び出し:**

Zig コアのキーバインド処理 → `rt_app.performAction(.new_tab)` → NotificationCenter → AppDelegate

```swift
// AppDelegate.swift
NotificationCenter.default.addObserver(
    self, selector: #selector(ghosttyNewTab(_:)),
    name: Ghostty.Notification.ghosttyNewTab, object: nil)

@objc private func ghosttyNewTab(_ notification: Notification) {
    guard let surfaceView = notification.object as? Ghostty.SurfaceView else { return }
    guard let window = surfaceView.window else { return }
    guard window.windowController is TerminalController else { return }  // QuickTerminal除外
    let config = notification.userInfo?[...] as? Ghostty.SurfaceConfiguration
    _ = TerminalController.newTab(ghostty, from: window, withBaseConfig: config)
}
```

**IBAction（メニューバー・ドックメニュー）からの呼び出し:**

```swift
@IBAction func newTab(_ sender: Any?) {
    _ = TerminalController.newTab(
        ghostty,
        from: TerminalController.preferredParent?.window
    )
}
```

**`preferredParent` の決定ロジック:**

```swift
static var preferredParent: TerminalController? {
    all.first { $0.window?.isMainWindow ?? false }
    ?? lastMain  // 最後にメインだったウィンドウ
    ?? all.last  // 最後のウィンドウ
}
```

これにより、Dock のクリックや App Intent からのアクションでも適切なウィンドウにタブが追加される。

### ウィンドウ管理（MainMenu でのタブ操作）

**タブ操作関連のメニューアイテム:**
- `menuNewTab` / `menuNewWindow`: 新規タブ/ウィンドウ
- `menuCloseTab`: タブを閉じる
- `menuCloseWindow`: ウィンドウ全体を閉じる
- `menuCloseAllWindows`: 全ウィンドウを閉じる

**Ghostty 設定とメニューショートカットの同期:**

```swift
private func syncMenuShortcuts(_ config: Ghostty.Config) {
    syncMenuShortcut(config, action: "new_window", menuItem: menuNewWindow)
    syncMenuShortcut(config, action: "new_tab", menuItem: menuNewTab)
    syncMenuShortcut(config, action: "close_tab", menuItem: menuCloseTab)
    ...
    // TerminalController 全てのタブラベルも更新
    TerminalController.all.forEach { $0.relabelTabs() }
}
```

キーバインド設定が変更されると全ウィンドウのタブラベルが再更新される（`goto_tab:1` 等のショートカットを各タブのアクセサリービューに表示するため）。

---

## SDITへの適用メモ

### SDIT の現在のサイドバー方式との差分

| 観点 | Ghostty | SDIT（目標） |
|---|---|---|
| タブバー位置 | 水平（上部） | 垂直（左側） |
| タブバー表示条件 | 2タブ以上で自動表示 | 2タブ以上で自動表示（同じ） |
| 1タブ時 | タブバー非表示 | タブバー非表示（SDI状態） |
| タブの実体 | 各タブ = 独立NSWindow | 各タブ = 独立winit Window（相当） |
| タブのドラッグ操作 | macOSネイティブタブに委任 | winit + objc2 で自前実装が必要 |
| タブ色 | TerminalTabColor（10色） | 将来的に対応可 |
| インラインタイトル編集 | NSTabBar（プライベートAPI） | 縦タブのため独自UIで実装 |

**最大の差分:** Ghostty は macOS の `NSWindowTabGroup` をそのまま使い、水平タブバーを得る。SDIT は縦タブを実装するため、macOS の横タブ機能を流用するのではなく、独自の縦タブ UI + `NSWindowTabGroup`（非表示）の組み合わせが必要。

### winit から NSWindow タブ API を呼ぶ方法（objc2 経由）

SDIT は Rust + winit + objc2 スタック。`NSWindowTabGroup` の API は以下のように呼べる:

```rust
// objc2 クレートを使う場合の擬似コード
use objc2_app_kit::NSWindow;
use objc2::rc::Retained;

// NSWindow.addTabbedWindow(_:ordered:)
let child_ns_window: Retained<NSWindow> = /* winit から取得 */;
let parent_ns_window: Retained<NSWindow> = /* winit から取得 */;
unsafe {
    parent_ns_window.addTabbedWindow_ordered(&child_ns_window, NSWindowOrderingMode::Above);
}

// tabGroup の取得
let tab_group = unsafe { parent_ns_window.tabGroup() };

// tabbedWindows の取得  
let tabbed_windows = unsafe { parent_ns_window.tabbedWindows() };
```

**winit から NSWindow を取得する方法:**

```rust
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

fn get_ns_window(window: &winit::window::Window) -> *mut objc2_app_kit::NSWindow {
    match window.window_handle().unwrap().as_raw() {
        RawWindowHandle::AppKit(handle) => handle.ns_window.as_ptr() as *mut _,
        _ => panic!("not AppKit"),
    }
}
```

**`addTabbedWindowSafely` の Rust 版:**

AppKit は `addTabbedWindow` が内部で Objective-C 例外を投げることがある（Ghostty のコメントより「visual tab picker flows」時）。Rust は ObjC 例外をキャッチできないため、ObjC ラッパー経由で呼ぶか、`objc2` の例外処理機能を使う必要がある。

### Session/Surface 分離で必要な変更

SDIT の設計では「Session（PTY）と Surface（描画先）を分離」とあるが、Ghostty のタブ実装から得られる知見:

**Ghostty の場合:** PTY と Surface は同じ `TerminalController` に属する。タブの「合体/切出し」は NSWindow レベルの操作なので PTY に影響しない。PTY を保持する `TerminalController` は変わらないから。

**SDIT の場合（推奨設計）:**

```
Session (PTY + ターミナル状態)
  ↑ 参照
Surface (描画先 = winit Window)

タブの合体:
  Session A の描画先 → 変わらない（Surface Aのまま）
  ただし縦タブバー付きのコンテナウィンドウに Surface が視覚的に収まる

タブの切出し:
  Surface を独立ウィンドウに移動
  Session は継続
```

実際には Ghostty 同様「1ウィンドウ = 1 Session = 1 Surface」のままで NSWindowTabGroup を使う方が実装が単純になる可能性がある。「表示先の差し替え」は macOS ネイティブタブが肩代わりしてくれるので。

### 注意点・落とし穴

1. **macOS の自動タブ挿入対策が必須**
   `windowDidLoad`（相当の初期化コールバック）で、macOS が勝手にタブグループに追加した場合を検知して除去する処理が必要。Ghostty は:
   ```swift
   if let tabGroup = window.tabGroup, tabGroup.windows.count > 1 {
       window.tabGroup?.removeWindow(window)
   }
   ```
   これを怠ると「システム設定でタブを常に開く」にしたユーザーで二重追加が起きる。

2. **`addTabbedWindow` は Objective-C 例外を投げる場合がある**
   Ghostty は `GhosttyAddTabbedWindowSafely`（ObjC ラッパー）を通して例外を捕捉する。Rust では ObjC 例外はキャッチ不可なので同様のラッパーが必要。

3. **タブの並び替え検知に標準 API がない**
   macOS はタブをドラッグで並べ替えたとき、デリゲートメソッドも通知も提供しない。Ghostty はアクセサリービューのフレーム変更通知 + ハッシュ比較というハックで対応。縦タブは独自 UI なのでこの問題は発生しないが、macOS ネイティブタブとの同期が必要な場合は注意。

4. **100ms デバウンスの `TabGroupCloseCoordinator` は重要**
   macOS の「赤×ボタン1回クリック」がタブグループ内の全ウィンドウに `windowShouldClose` を連続送信する挙動は、SDIT でも同様に発生する可能性がある。「タブを閉じる」vs「ウィンドウ全体を閉じる」の判定に同様のコーディネータが必要。

5. **タブラベルのキーショートカット表示**
   `goto_tab:1` 〜 `goto_tab:9` のキーバインドが変更されたとき、全タブのアクセサリービューを更新する必要がある。キーバインド設定変更の通知を購読し、全ウィンドウを走査して更新する設計が必要。

6. **縦タブと macOS ネイティブタブを組み合わせる場合の表示**
   SDIT が「縦タブ = 独自 SwiftUI/AppKit ビュー」と「NSWindowTabGroup（水平タブバー非表示）」を組み合わせる設計にする場合、macOS の水平タブバーを完全に非表示にする手段が必要。`NSWindow.tabbingMode = .disallowed` にするとタブグループ機能自体が使えなくなるので、タブバーを非表示にしながらグループ機能は使う工夫が必要。Ghostty のタイトルバースタイル `tabs` バリアント（`TerminalTabsTitlebarVentura`/`TerminalTabsTitlebarTahoe`）が参考になる可能性がある。

7. **Undo/Redo のためのタブ状態保存**
   Ghostty は `UndoState` にタブグループへの `weak` 参照とタブインデックスを保存し、Undo 時に元のタブグループ・位置に復元する。SDIT でも同様の仕組みが必要な場合はこのアプローチが参考になる。
