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

**コントリビューションモデル:**
- コードではなく計画（Plan）をレビューする。筋の良い計画は受け入れ、実装はAIが担保する
- 派生Forkの改善は積極的にアップストリームへ取り込む（GPLv3）
- CLAUDE.mdを改善した場合は、CONTRIBUTING.mdを読んでオーナーへの確認を必ず行うこと
- 詳細は [CONTRIBUTING.md](./CONTRIBUTING.md) を参照

**ブートシーケンス（セッション開始時に必ず実行）:**
1. `docs/knowhow/` を読む — 過去の実装知見・注意点を把握する
2. `.claude/Feedback.md` を読む — 未対応の改善アクションを確認する
3. `TODO.md` を読む — タスクバックログと優先度を把握する
4. `.claude/Progress.md` を読む — 現在進行中のタスクがあれば継続する

**開発プロセスファイル一覧:**
- **@TODO.md**: タスクバックログ。`docs/plans/` の完了状態・優先度を追跡する
- **@.claude/Progress.md**: 現在のタスク進捗。運用ルールもここに記載
- **@.claude/Feedback.md**: 問題・改善アクションの記録
- `docs/plans/`: 実装計画ファイル
- `docs/knowhow/`: 実装で得たノウハウの蓄積
- `docs/ref-notes/`: リファレンス読解メモ・外部ドキュメント

---

## リファレンスプロジェクト一覧

| # | プロジェクト | 言語 | 参照先 |
|---|---|---|---|
| 1 | Alacritty | Rust | `refs/alacritty/` |
| 2 | Ghostty | Zig | `refs/ghostty/` |
| 3 | WezTerm | Rust | `refs/wezterm/` |
| 4 | Zellij | Rust | `refs/zellij/` |

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
- macOS/X11ウィンドウ生成コード → winit抽象に統一する

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
- macOS AppKit統合コード（Ghostty固有）
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
│          ↑ wgpu rendering (Ghostty参照)                  │
│                                                         │
│  ┌──────────────────────────────────────────────────┐   │
│  │  SessionSidebar (縦タブ) — デフォルト非表示       │   │
│  │  Zellij tab-bar plugin参照                       │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
          ↓
┌─────────────────────────────────────────────────────────┐
│  sdit-core (library crate)  ← Ghostty libghostty参照    │
│                                                         │
│  Terminal  PTY  Grid  VTE-Parser  FontRasterizer        │
│  ↑Ghostty参照  ↑Alacritty参照    ↑Ghostty参照           │
└─────────────────────────────────────────────────────────┘
          ↓
┌─────────────────────────────────────────────────────────┐
│  sdit-session (library crate) ← WezTerm Mux参照         │
│                                                         │
│  SessionManager  WindowRegistry  ArrangementStore       │
│  ← WezTerm mux参照              ← Zellij session参照    │
└─────────────────────────────────────────────────────────┘
```

---

## クレート構成

```
sdit/
├── crates/
│   ├── sdit/              # バイナリ。GUIループ・ウィンドウ生成
│   │   └── src/main.rs
│   ├── sdit-core/         # PTY・VTE・グリッド・フォント。GUIゼロ依存
│   │   └── src/
│   │       ├── terminal/  # VTEステートマシン
│   │       ├── grid/      # セルグリッド・スクロールバック
│   │       ├── pty/       # PTYプロセス管理
│   │       └── font/      # フォントラスタライズ
│   ├── sdit-session/      # セッション・ウィンドウ状態管理
│   │   └── src/
│   │       ├── session.rs        # Session型
│   │       ├── window_registry.rs # SDIウィンドウ一覧
│   │       └── sidebar.rs        # 縦タブ状態
│   ├── sdit-render/       # wgpuレンダーバックエンド
│   │   └── src/
│   │       ├── pipeline.rs
│   │       └── atlas.rs   # フォントテクスチャアトラス
│   └── sdit-config/       # TOML設定スキーマ
│       └── src/
├── refs/                  # リファレンスOSS（git submodule）
└── docs/                  # 読解メモ・計画ファイル
```

---

## 実装ロードマップ

各フェーズの詳細タスクは `docs/plans/` を参照。進捗は `TODO.md` で追跡する。

| Phase | 名称 | 概要 | 計画ファイル |
|---|---|---|---|
| 0 | リファレンス読解 | 4プロジェクトのソースを読み設計知見を蓄積 | `docs/plans/phase0-reference-reading.md` |
| 1 | sdit-core MVP | PTY・VTE・グリッドのヘッドレス実装 | `docs/plans/phase1-core-mvp.md` |
| 2 | 最初のSDIウィンドウ | winit + wgpu で1枚表示・PTY接続 | `docs/plans/phase2-first-sdi-window.md` |
| 3 | SDI本実装 | 複数ウィンドウ・セッション管理 | `docs/plans/phase3-sdi-multi-window.md` |
| 4 | 縦タブ（SessionSidebar） | Chrome-like 合体・切出しUX | `docs/plans/phase4-session-sidebar.md` |
| 5 | 設定・仕上げ | TOML設定・永続化・フォント | `docs/plans/phase5-config-polish.md` |

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

```toml
# 確定採用
vte = "0.13"          # VTEパーサー (Alacrittyも使用)
pty-process = "0.4"   # PTYプロセス管理 (Alacrittyより)
winit = "0.30"        # ウィンドウ管理
wgpu = "0.20"         # GPUレンダリング
cosmic-text = "0.12"  # フォントシェーピング (Ghostty方式代替)
serde = "1"           # 設定シリアライズ
toml = "0.8"          # 設定ファイル

# 検討中
glutin = ?            # wgpuで代替可能か検討
tokio = ?             # 非同期PTY読み取りに必要か検討
```

---

## コミットログ規約

日本語で書く。形式:

```
[領域] 変更内容の要約

詳細説明（必要な場合）

参照: refs/alacritty/alacritty-terminal/src/grid/mod.rs
```

領域プレフィックス:
- `[core]` sdit-core への変更
- `[render]` sdit-render への変更
- `[session]` sdit-session への変更
- `[gui]` sdit バイナリのGUI変更
- `[config]` sdit-config への変更
- `[ref]` リファレンス読解メモ・設計ドキュメント更新
- `[arch]` アーキテクチャ上の決定変更

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
