# CLAUDE.md — SDIT Terminal プロジェクト設計指針

> このファイルはAIエージェント（Claude等）がコードを書く際に従うべき
> アーキテクチャ判断・参照先・禁止事項を定めたドキュメントです。
> 人間のコントリビューターも必ず一読してください。

---

## プロジェクト憲法

**SDIT** は「SDIファースト、縦タブセカンド」のターミナルエミュレータです。

```
セッションは本来バラバラに存在する。
束ねたくなったときだけ縦タブが出現する。
解きたくなったらドラッグで元に戻る。（Chrome-like UX）
```

この哲学に反する変更は、どれだけ実装が優れていても却下します。

**プラットフォーム方針:**
- 過度にクロスプラットフォームにこだわらない。macOS 向けの最適化（パフォーマンス、UX）ができるならそちらを優先する
- OS 別に分岐する仕様・コードは DI のように差し替えられる余地を残した設計にする
- 他 OS 対応は OS 別設計書として概要だけ `docs/plans/` に残し、着手せず置いておくこと

**コントリビューションモデル:**
- コードではなく計画（Plan）をレビューする。筋の良い計画は受け入れ、実装はAIが担保する
- 派生Forkの改善は積極的にアップストリームへ取り込む（GPLv3）
- CLAUDE.mdを改善した場合は、CONTRIBUTING.mdを読んでオーナーへの確認を必ず行うこと
- 詳細は [CONTRIBUTING.md](./CONTRIBUTING.md) を参照

**ブートシーケンス（セッション開始時に必ず実行）:**
1. `.claude/Feedback.md` を読む — 未対応の改善アクションを確認する
2. `TODO.md` を読む — タスクバックログと優先度を把握する
3. `.claude/Progress.md` を読む — 現在進行中のタスクがあれば継続する
4. **TODO に未完了タスクがない場合** — 機能発見フローを実行する:
   - `docs/knowhow/feature-survey-log.md` を読み、前回の調査済み領域を確認する
   - リファレンス実装（`refs/`）の **未調査領域** を差分調査し、以下を発見する:
     a. **当たり前品質**: ユーザーが常用するなら「ないと乗り換えがつらい」機能
     b. **あったら便利**: 他ターミナルにあって SDIT にない便利機能
   - 発見した機能を `docs/plans/` に Plan として作成する
     - **関心事が異なる機能は別の計画ファイルにする**（例: `phase13.2-visual-bell.md` と `phase13.3-window-opacity.md` は別ファイル）。1つの `phaseX.Y` に複数の関心事を詰め込まない
   - TODO.md に組み入れる
   - 調査した領域を `docs/knowhow/feature-survey-log.md` に追記する
   - 完了条件: Plan 作成 + TODO 組み入れ + 調査ログ更新
5. 次に着手する Plan を特定したら、knowhow サブエージェントを起動する:
   - Plan ファイルの内容をサブエージェントに渡す
   - サブエージェントは `docs/knowhow/` を全読みし、Plan に必要・有用なノウハウのパスと要約をメインコンテキストに返す
   - メインコンテキストには Plan に関連する knowhow のみが載り、コンテキスト消費を抑制する

**開発プロセスファイル一覧:**
- **@TODO.md**: タスクバックログ。`docs/plans/` の完了状態・優先度を追跡する
- **@.claude/Progress.md**: 現在のタスク進捗。運用ルールもここに記載
- **@.claude/Feedback.md**: 問題・改善アクションの記録
- `docs/plans/`: 実装計画ファイル
- `docs/knowhow/`: 実装で得たノウハウの蓄積
- `docs/ref-notes/`: リファレンス読解メモ・外部ドキュメント

---

## リファレンスプロジェクト一覧

| # | プロジェクト | 言語 | 参照先 | 参照用途 |
|---|---|---|---|---|
| 1 | Alacritty | Rust | `refs/alacritty/` | コード参照 |
| 2 | Ghostty | Zig | `refs/ghostty/` | コード参照 |
| 3 | WezTerm | Rust | `refs/wezterm/` | コード参照 |
| 4 | Zellij | Rust | `refs/zellij/` | コード参照 |
| 5 | Wave Terminal | Go/TypeScript | `refs/waveterm/` | 設計思想参照（ブロック UI・入力フォーム独立設計） |
| 6 | DomTerm | JavaScript/C | `refs/domterm/` | 設計思想参照（shell integration による入出力分離） |

**参照用途の区分:**
- **コード参照**: ソースコードを直接読み、実装パターンやアーキテクチャを SDIT に取り入れる
- **設計思想参照**: コードの直接流用ではなく、設計思想・UX パターン・アーキテクチャの考え方を参考にする。Plan 作成時や機能発見フローで、これらのプロジェクトの設計判断を引用・比較してよい

サブモジュールの初期化:
```bash
git submodule update --init --depth=1
```

---

## 参照指針：プロジェクト別

---

### 1. Alacritty — PTYコアと「シンプルを守る哲学」

**参照パス:**
```
refs/alacritty/alacritty/src/         # アプリケーションエントリーポイント
refs/alacritty/alacritty-terminal/src/ # PTY・VTE・グリッドのコア実装
refs/alacritty/alacritty-config/src/  # 設定スキーマの設計
```

**取り入れるもの:**

| 領域 | 対象ファイル | 取り入れる理由 |
|---|---|---|
| PTYプロセス管理 | `alacritty-terminal/src/tty/` | `pty-process` クレート活用パターン |
| VTEパーサー統合 | `alacritty-terminal/src/ansi.rs` | ANSIシーケンス処理の網羅性 |
| グリッドデータ構造 | `alacritty-terminal/src/grid/` | セル・行・スクロールバック設計 |
| イベントループ | `alacritty/src/event.rs` | winit統合の参照実装 |

**取り入れない（重要）:**
- タブ・スプリット関連のコードは存在しないため参照不要
- OpenGLレンダラー → wgpuで独自実装するため不採用
- macOS 固有のウィンドウ管理コードは参考にしてよい（プラットフォーム方針参照）

**学ぶ哲学:**
> 「機能追加の要求を断る勇気」— Alacrittyのissueを読んで
> 「なぜこれを実装しないか」の理由付けを学ぶ。
> SDITも同様に、SDIを壊す機能要求は断る。

---

### 2. Ghostty — アーキテクチャ分離と高速レンダリング

**参照パス:**
```
refs/ghostty/src/                     # Zigコア実装全体
refs/ghostty/src/terminal/            # ターミナルステートマシン
refs/ghostty/src/renderer/            # Metal/Vulkan/DirectX分岐
refs/ghostty/src/Surface.zig          # サーフェス管理（最重要）
refs/ghostty/src/App.zig              # アプリケーション構造
```

**取り入れるもの:**

| 領域 | 対象ファイル | 取り入れる理由 |
|---|---|---|
| コア/GUI分離設計 | `src/App.zig`, `src/Surface.zig` | libghostty方式の分離アーキテクチャ |
| ターミナルステート | `src/terminal/Terminal.zig` | VT状態管理の完全な実装 |
| フォントシェーピング | `src/font/` | サブピクセルレンダリング手法 |
| レンダーパイプライン概念 | `src/renderer/` | GPU描画の抽象化レイヤー設計 |
| サーフェス概念 | `src/Surface.zig` | 1サーフェス=1セッションの対応 |

**SDITへの適用:**

```
Ghosttyの「libghostty」思想をRustで再現する:

sdit-core (ライブラリクレート)
  ↓ pub API
sdit (バイナリクレート)  ←→  OS GUI レイヤー

sdit-coreはGUIに依存しない。
テスト・ヘッドレス実行・将来のSSHリモートレンダリングに対応できる。
```

**取り入れない:**
- Zigコード自体はそのままでは使えない（言語が違う）
- macOS AppKit統合コードは直接流用しないが、macOS 最適化の設計参考としては参照可（プラットフォーム方針参照）
- ZigのCインターフェース生成部分

---

### 3. WezTerm — ウィンドウ状態管理とSDI実装

**参照パス:**
```
refs/wezterm/wezterm-gui/src/         # GUIメインループ・ウィンドウ管理
refs/wezterm/wezterm-gui/src/glwindow.rs  # ウィンドウ1枚の実装
refs/wezterm/wezterm-mux/src/         # セッション多重化レイヤー
refs/wezterm/config/src/              # Lua設定エンジン
refs/wezterm/wezterm-client/src/      # クライアント/サーバー分離
```

**取り入れるもの:**

| 領域 | 対象ファイル | 取り入れる理由 |
|---|---|---|
| ウィンドウライフサイクル | `wezterm-gui/src/glwindow.rs` | SDIウィンドウ1枚の生成・破棄・再利用 |
| Muxレイヤー設計 | `wezterm-mux/src/` | セッション≠ウィンドウの分離モデル |
| タブバーレンダリング | `wezterm-gui/src/tabbar.rs` | 縦タブUIの**逆参照**（横タブを縦に変換する思考実験） |
| 設定スキーマ設計 | `config/src/` | TOML設定のデフォルト値・バリデーション設計 |
| クライアント通信 | `wezterm-client/src/` | 将来のマルチプロセス構成の参考 |

**SDIへの適用（最重要参照）:**

WezTermは「1ウィンドウ = N タブ」が常態だが、
SDITは「1タブ時はタブバー消滅 = SDI状態がデフォルト」。

```
WezTerm: Window → TabBar(常時表示) → [Tab, Tab, Tab] → Pane
SDIT:    Window → TabBar(2タブ以上で出現) → [Tab, Tab] → Session
         Window → (1タブ → タブバー非表示) → Session  ← SDI状態
```

Session/Surface分離（Ghostty参照）:
  Session = PTY + Terminal状態。ウィンドウとは独立して生存
  Surface = 描画先。合体・切出し時に差し替え。PTYは切れない

**取り入れない:**
- 水平タブバーの実装（設計が逆）
- Lua設定エンジン（TOMLで十分。複雑性を避ける）
- wgpu以外のレンダーバックエンド

---

### 4. Zellij — セッション管理と縦タブUI

**参照パス:**
```
refs/zellij/zellij-server/src/        # セッションサーバー実装
refs/zellij/zellij-client/src/        # クライアント側レンダリング
refs/zellij/zellij-tile/src/          # プラグインシステム
refs/zellij/zellij-utils/src/         # 共有型定義・プロトコル
refs/zellij/default-plugins/tab-bar/  # タブバープラグイン実装（最重要）
```

**取り入れるもの:**

| 領域 | 対象ファイル | 取り入れる理由 |
|---|---|---|
| タブバープラグイン | `default-plugins/tab-bar/src/` | セッションリスト表示・選択UIの参考 |
| セッション状態型 | `zellij-utils/src/data.rs` | SessionInfo, TabInfo等の型設計 |
| クライアント/サーバー分離 | `zellij-server/`, `zellij-client/` | セッション永続化アーキテクチャ |
| キー入力処理 | `zellij-client/src/os_input_output.rs` | 生のターミナル入力処理 |

**縦タブへの適用:**

```
Zellijのタブバーは「常に表示」が前提だが、
SDITでは「縦タブバー」として再設計する:

- タブが1つのとき: タブバー非表示（= SDI状態）
- タブが2つ以上のとき: 縦タブバーが自動出現
- Cmd+\ でも手動トグル可能

■ Chrome-like UX（組み入れ・汲み出し）
  組み入れ: SDIウィンドウを別のウィンドウにドラッグ → タブとして合体
  汲み出し: 縦タブをドラッグアウト → 独立したSDIウィンドウに復帰
  → セッション（PTY）は切れない。表示先（Surface）を差し替えるだけ

■ 状態遷移
  [Win A] [Win B]  →(合体)→  [Win: TabA|TabB]  →(切出し)→  [Win A] [Win B]
   タブバーなし                 縦タブバー出現                  タブバー消滅

Zellijのタブ概念 → SDITの概念への変換表:
  Tab   → Session（縦タブの各項目）
  Pane  → 存在しない（分割はしない）
  Layout → WindowArrangement（ウィンドウの配置記憶）
```

**取り入れない:**
- ペイン分割システム全体（SDIT はペイン分割しない）
- WebAssemblyプラグインシステム（初期バージョンでは不要）
- TUIレンダリング層（SDITはネイティブ描画）

---

### 5. Wave Terminal — ブロック UI と入力フォーム独立設計

**参照用途:** 設計思想参照（コード直接参照ではない）

**参照パス:**
```
refs/waveterm/                        # Go/TypeScript ベースのターミナル
```

**取り入れる設計思想:**

| 領域 | 参考にする思想 |
|---|---|
| ブロック UI | ターミナル出力をブロック単位で構造化する考え方 |
| 入力フォーム独立設計 | コマンド入力領域をターミナルグリッドと独立させる設計 |

**取り入れない:**
- Go/TypeScript の実装（言語が異なる）
- Web ベースのレンダリング方式
- ブロック UI の実装そのもの（SDI 哲学との整合性を検討してから）

---

### 6. DomTerm — Shell Integration による入出力分離

**参照用途:** 設計思想参照（コード直接参照ではない）

**参照パス:**
```
refs/domterm/                         # JavaScript/C ベースのターミナル
```

**取り入れる設計思想:**

| 領域 | 参考にする思想 |
|---|---|
| Shell Integration | OSC シーケンスによるプロンプト認識・入出力分離 |
| コマンド出力の構造化 | 出力ブロックの折りたたみ等の UX |

**取り入れない:**
- DOM ベースのレンダリング
- JavaScript の実装

---

## SDIT アーキテクチャ全体像

```
┌─────────────────────────────────────────────────────────┐
│  sdit (binary)                                          │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  Window 1    │  │  Window 2    │  │  Window 3    │  │
│  │  (SDI)       │  │  (SDI)       │  │  (SDI)       │  │
│  │              │  │              │  │              │  │
│  │  Session A   │  │  Session B   │  │  Session C   │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
│          ↑ wgpu rendering                               │
│                                                         │
│  ┌──────────────────────────────────────────────────┐   │
│  │  SessionSidebar (縦タブ) — デフォルト非表示       │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
          ↓
┌─────────────────────────────────────────────────────────┐
│  sdit-core (library crate)                              │
│                                                         │
│  terminal/ pty/ grid/ font/  ← PTY・VTE・Grid コア      │
│  render/                     ← wgpu パイプライン・アトラス│
│  session/                    ← セッション管理・永続化    │
│  config/                     ← TOML 設定・カラーテーマ   │
└─────────────────────────────────────────────────────────┘
```

---

## クレート構成

主要ファイルのみ記載。全量は `ls crates/sdit/src/` / `ls crates/sdit-core/src/` で確認すること。

```
sdit/
├── crates/
│   ├── sdit/              # バイナリ。GUIループ・ウィンドウ生成
│   │   └── src/           # main.rs, app.rs, event_loop.rs, input.rs,
│   │                      # window.rs, window_ops.rs, render.rs 等
│   └── sdit-core/         # ライブラリ。PTY/VTE/Grid/Render/Session/Config
│       └── src/
│           ├── terminal/  # VTEステートマシン
│           ├── grid/      # セルグリッド・スクロールバック
│           ├── pty/       # PTYプロセス管理
│           ├── font/      # フォントラスタライズ（低レベル）
│           ├── render/    # wgpuレンダー（pipeline/atlas/font context）
│           ├── session/   # セッション管理・サイドバー状態・永続化
│           ├── config/    # TOML設定スキーマ・カラーテーマ
│           ├── selection.rs  # テキスト選択
│           └── index.rs   # セルインデックス型
├── tools/test-utils/      # GUI テスト補助ツール
├── docs/test-scenarios/   # 統合テストシナリオ
├── refs/                  # リファレンスOSS（git submodule）
└── docs/                  # 読解メモ・計画ファイル
```

---

## セキュリティレビュー方針

ターミナルエミュレータは外部入力（エスケープシーケンス等）を大量に処理するため、
セキュリティレビューを開発プロセスに組み込む。

**攻撃面:**
- VTE パーサー: 悪意あるエスケープシーケンスによるバッファ操作・状態破壊
- PTY I/O: 入出力バッファの境界チェック
- TOML 設定パース: デシリアライゼーション安全性
- フォントファイル処理: 外部ファイルパース

**プロセス:**
1. **計画レビュー（全フェーズ）**: Plan 作成時にセキュリティサブエージェントを起動し、設計上の脆弱性を指摘・修正案を提示する（実装はしない）
2. **コードレビュー（コード変更時）**: タスク完了時にセキュリティサブエージェントを起動し、実装コードの脆弱性を指摘・修正案を提示する（実装はしない）
3. **ペネトレーションテスト（バイナリ動作時）**: VTE ファジング等のペネトレーションテストを計画・実施する

**修正フロー:**
1. レビューで脆弱性が発見されたら、Progress のチェックリストにサブタスクとして追加する
2. **重要度別の対処ルール（先送り禁止基準）:**
   - **Critical / High**: そのフェーズ内で必ず修正する。先送り不可
   - **Medium**: そのフェーズ内で修正する。先送りする場合は「影響範囲が限定的」等の具体的理由を Plan に記載し、`docs/plans/phaseX.Y-security-fixes.md` として独立した計画を起こす（次フェーズに関係ないタスクを埋め込まない）
   - **Low / Info**: Plan に記録し、必要に応じて `phaseX.Y` の独立計画で対応する
3. 修正したサブタスクを Progress 上で完了チェックする
4. **セキュリティ修正が全て完了するまでフェーズの完了コミットを行わない**
5. 修正結果を該当フェーズの **Plan ファイル** (`docs/plans/`) に記録する
6. Feedback.md にはセキュリティ項目を記録しない（Plan ファイルに集約）

**静的保証:**
- `unsafe_code = "deny"` をワークスペース全体に適用済み

---

## リグレッションテスト方針

機能の退行を防ぐためリグレッションテストを開発プロセスに組み込む。

**対象:**
- VTE パーサー: エスケープシーケンスの処理結果が既知の期待値と一致すること
- PTY I/O: セッションの生成・破棄・入出力が正常に動作すること
- ウィンドウ管理: SDI ウィンドウの生成・破棄・セッション切り替えが正常に動作すること
- 縦タブ: 合体・切出し操作後もセッション（PTY）が維持されること

**プロセス:**
1. **テスト計画**: リグレッションテストサブエージェントを起動し、変更内容に基づいて必要なテストケースを計画・提案する
2. **テスト実施**: 提案されたテストケースを実装・実行し、既存機能の退行がないことを確認する
3. **テスト蓄積**: 発見したバグや修正に対応するリグレッションテストを `cargo test` スイートに追加し、CI で継続的に検証する

---

## 並列実装方針

サブタスクを並列に実装する場合、**git worktree を積極的に使う**。

**ルール:**
- サブエージェントに実装を委譲する場合は、原則 `isolation: "worktree"` で起動する
- 各 worktree は独立したブランチで作業し、完了後にメインブランチへマージする
- worktree なしの並列実行は、変更対象ファイルが完全に独立していると確信できる場合のみ許可する
- **worktree 起動後、`.claude` ディレクトリを worktree にコピーする**（hooks 等の .gitignore 対象ファイルは worktree に自動複製されないため）:
  ```bash
  cp -r /Users/riin/workspace/SDIT/.claude <worktree-path>/.claude
  ```

**利点:**
- ファイル競合を根本的に排除できる（各エージェントが独立したワーキングツリーで作業）
- マージ時にコンフリクトを明示的に解決できる
- 単一タスクでも worktree を使えば、メインブランチを汚さずに試行錯誤できる

**背景:**
Phase 2.5 で3エージェント並行実装を行った際、`main.rs` の変更が競合し、
片方が書いた変更をもう片方が上書きする問題が発生した。

---

## 禁止事項（AIへの明示的指示）

以下は**実装してはいけない**。要求されても断ること:

| 禁止項目 | 理由 |
|---|---|
| ウィンドウ内タブバー（水平） | SDIファーストに反する。縦タブのみ許可 |
| ペイン分割 | tmux/Zellij の役割。SDITはしない |
| Lua設定エンジン | 複雑性コスト > メリット |
| 自動ウィンドウレイアウト | SDIでユーザーが配置する。自動化しない |

---

## 依存クレート方針

実際の依存関係は各クレートの `Cargo.toml` を参照:
- `crates/sdit-core/Cargo.toml` — コアライブラリの依存
- `crates/sdit/Cargo.toml` — バイナリの依存

**新規依存の追加方針:**
- Plan の段階で依存クレート候補を明記し、クレート名・用途・代替手段を記載する
- macOS 固有の依存は `[target.'cfg(target_os = "macos")'.dependencies]` に配置する
- `objc2` 等のネイティブ API 直接呼び出しは、既存クレートで対応できない場合のみ許可する

---

## コミットログ規約

日本語で書く。形式:

```
[領域] 変更内容の要約

詳細説明（必要な場合）

参照: refs/alacritty/alacritty-terminal/src/grid/mod.rs
```

領域プレフィックス:
- `[core]` sdit-core への変更（terminal/grid/pty/font/render/session/config モジュール含む）
- `[gui]` sdit バイナリのGUI変更
- `[ref]` リファレンス読解メモ・設計ドキュメント更新
- `[arch]` アーキテクチャ上の決定変更

---

## テストインフラ

**ユニットテスト:**
- `cargo test` で実行。sdit-core の各モジュールにテストを配置
- ヘッドレステスト: `crates/sdit/src/headless.rs` で GUI なしのスモークテスト

**GUI 統合テスト:**
- `tools/test-utils/` — GUI テスト補助ツール群（スクリーンショット撮影、OCR、ウィンドウ情報取得等）
  - `capture-window.swift` — ウィンドウキャプチャ
  - `window-info.swift` — ウィンドウ位置・サイズ取得
  - `verify-text.swift` — テキスト検証（Vision OCR）
  - `send-keys.sh` — キー入力送信
  - 詳細は `tools/test-utils/README.md` を参照
- `docs/test-scenarios/` — 統合テストシナリオ定義
  - 各シナリオは Markdown で手順・期待結果を記述
  - `docs/test-scenarios/INDEX.md` でシナリオ一覧と最終実行日を管理
  - `/gui-test` スキルで SDIT バイナリを起動してスクリーンショット検証を実行

---

## macOS 固有依存の方針

- macOS 向けの最適化ができるならそちらを優先する（プラットフォーム方針）
- `#[cfg(target_os = "macos")]` で分岐し、`Cargo.toml` では `[target.'cfg(target_os = "macos")'.dependencies]` に配置する
- `objc2` 等のネイティブ API 直接呼び出しは、既存クレート（`global-hotkey`, `muda` 等）で対応できない場合のみ許可する
- 他 OS 対応用の分岐ポイントを設計に残し、将来の DI 的差し替えに備える

---

## リファレンス・ドキュメントの管理

`docs/ref-notes/` はリファレンスに関する知識の蓄積先:

- **読解メモ**: サブモジュールのソースコードを読んだ記録
- **外部ドキュメント**: WebFetchした公式ドキュメントやライブラリAPIドキュメントもここに保存する

命名規約:
- 読解メモ: `{プロジェクト名}-{対象}.md`（例: `alacritty-grid.md`）
- 外部ドキュメント: `{ライブラリ名}-api.md`（例: `wgpu-api.md`, `vte-api.md`）

読解メモの書き方:

```markdown
# alacritty: grid/mod.rs 読解メモ

## 発見した設計
- Row<T>はVec<Cell>のnewtype
- スクロールバックはRaw<Row>のVecDeque

## SDITへの適用
- 同じ構造を sdit-core/src/grid/ に採用する
- ただしスクロールバックの上限設定をTOMLで変更可能にする

## 疑問点
- Alacrittyはスクロールバックをどこでトリミングしている?
  → grid/mod.rs:312 `fn truncate` 参照
```
