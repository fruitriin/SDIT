# Phase 5.5〜12 ロードマップ — 日常使いターミナルへの道

## 設計方針

現在のSDITは「表示」は基本的に動くが、「操作」面で致命的な欠落がある。

**優先順位の根拠: 当たり前品質分析**

リファレンス実装(Alacritty, Ghostty, WezTerm)の全てが実装しており、
ないとシェルやTUIアプリが正常動作しない機能を「当たり前品質」と定義する。
これらを最優先で実装し、その後にマウス・選択等の操作機能を積み上げる。

```
Phase 5.5: ターミナル互換性（シェルが壊れる問題の修正）
    ↓
Phase 6: マウス・選択・クリップボード（TUIアプリの日常使い）
    ↓
Phase 7+: IME、UI改善、配布
```

---

## Phase 5.5: ターミナル互換性基盤

**概要**: シェル(bash/zsh/fish)やTUIアプリ(vim/htop/lazygit)が正常動作するために必須の、
小工数だが致命的に不足しているVTEシーケンスとキー入力処理を実装する。

### Phase 5.5.1: デバイスレポート + カーソルスタイル

| タスク | 詳細 | 変更先 | 工数 |
|---|---|---|---|
| DA1（Primary Device Attributes） | `CSI c` / `CSI 0 c` → `CSI ? 62 ; 4 c` を応答（VT220互換） | sdit-core (`handler.rs`) | 極小 |
| DA2（Secondary Device Attributes） | `CSI > c` → `CSI > 0 ; version ; 0 c` を応答 | sdit-core (`handler.rs`) | 極小 |
| DSR（Device Status Report） | `CSI 5 n` → `CSI 0 n`（端末OK）を応答 | sdit-core (`handler.rs`) | 極小 |
| CPR（Cursor Position Report） | `CSI 6 n` → `CSI row ; col R` を応答。**fish/zshプロンプト描画が依存** | sdit-core (`handler.rs`) | 極小 |
| DECSCUSR（カーソルスタイル変更） | `CSI n SP q` でカーソル形状変更: 0-2=Block, 3-4=Underline, 5-6=Bar。奇数=点滅、偶数=固定 | sdit-core (`terminal/mod.rs`, `handler.rs`) | 小 |
| カーソルスタイルのレンダリング | Block/Underline/Bar の3形状を描画。点滅はタイマーで制御 | sdit-render (`pipeline.rs`), sdit (`main.rs`) | 中 |

**DA/DSR/CPR の緊急度**: fish はDA応答がないとフォールバックモードに入り警告を出す。
zsh のプロンプトはCPRでカーソル位置を取得して描画位置を決定する。

**リファレンス**:
- `refs/alacritty/alacritty_terminal/src/ansi.rs` — Handler trait の device_status, primary_da 等
- `refs/ghostty/src/terminal/modes.zig` — cursor_visible (mode 25), cursor_blinking (mode 12)
- `refs/ghostty/src/terminal/cursor.zig` — CursorStyle 定義

### Phase 5.5.2: Alt→ESC + ベル + タイトル反映

| タスク | 詳細 | 変更先 | 工数 |
|---|---|---|---|
| Alt→ESC prefix | Alt+文字 入力時に `ESC` + 文字 をPTYに送信。**vim/emacs/htopの基本操作が依存** | sdit (`main.rs` key_to_bytes) | 小 |
| macOS Option キー設定 | Option を Alt として扱うか、特殊文字入力に使うか選択可能に | sdit-config, sdit (`main.rs`) | 小 |
| ベル通知（BEL 0x07） | BEL受信時にイベント発火。ビジュアルベル（画面フラッシュ）を実装 | sdit-core (`handler.rs`), sdit (`main.rs`) | 小 |
| ウィンドウタイトル反映 | Terminal::title() の値を `window.set_title()` でOSウィンドウに反映 | sdit (`main.rs`) | 極小 |
| カーソルブリンクモード | Mode 12 (AT&T 610 cursor blinking) のDECSET/DECRST対応 | sdit-core (`handler.rs`) | 極小 |

**Alt→ESC の緊急度**: Alt+キーが効かないと vim の `<M-...>` マッピング、
emacs の `M-x`、htop のメニュー操作等が全て使えない。

**リファレンス**:
- `refs/alacritty/alacritty/src/input/mod.rs` — alt_send_esc() 実装
- `refs/ghostty/src/terminal/modes.zig` — alt_esc_prefix (mode 1036)
- `refs/wezterm/config/src/bell.rs` — VisualBell 設定

**依存関係**: なし（Phase 5完了後すぐに着手可能）

**見積もり**: 全タスク合わせて数時間程度。工数に対してインパクトが極めて大きい。

---

## Phase 6: マウス・選択・クリップボード基盤

### Phase 6.1: マウスイベント報告 + スクロールUI

**概要**: TUIアプリ(vim, htop, lazygit等)の日常使いに必須のマウスイベント報告とスクロールUIを実装する。

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| TermMode にマウスモード追加 | `MOUSE_REPORT_CLICK`, `MOUSE_REPORT_DRAG`, `MOUSE_REPORT_MOTION`, `SGR_MOUSE`, `UTF8_MOUSE` を `TermMode` bitflags に追加 | sdit-core |
| CSI DECSET/DECRST にマウスモード処理追加 | `?9`, `?1000`, `?1002`, `?1003`, `?1006` の set/reset を `set_private_mode` に追加 | sdit-core (`handler.rs`) |
| main.rs にマウスイベントディスパッチ追加 | `WindowEvent::MouseInput`, `CursorMoved`, `MouseWheel` をマウスモードに応じてPTYにSGR/X11形式で報告 | sdit (`main.rs`) |
| ビューポートスクロール | マウスホイール(マウスモードOFF時)・Shift+PageUp/Down でスクロールバック閲覧 | sdit-core (`grid/mod.rs`), sdit (`main.rs`) |

**依存関係**: なし

**リファレンス**:
- `refs/alacritty/alacritty_terminal/src/term/mod.rs` — TermMode のマウスフラグ定義
- `refs/alacritty/alacritty/src/input/mod.rs` — マウスイベントからPTYバイト列への変換

### Phase 6.2: テキスト選択 + クリップボード

**概要**: テキストの選択・コピー・ペースト操作を実装する。

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| Selection 型の実装 | `SelectionRange`(start/end Point + block mode)を追加 | sdit-core (新規 `selection.rs`) |
| マウスドラッグでの選択 | 左ボタン押下で選択開始、ドラッグで範囲拡大、ダブルクリックで単語選択、トリプルクリックで行選択 | sdit (`main.rs`) |
| 選択範囲のレンダリング | 選択セルの前景/背景色を反転して描画 | sdit-render (`pipeline.rs`) |
| クリップボード統合 | `arboard` クレート使用。Cmd+C でコピー、Cmd+V でペースト(BRACKETED_PASTE対応) | sdit (`main.rs`) |
| OSC 52 クリップボード操作 | アプリ側からのクリップボード操作を処理 | sdit-core (`terminal/mod.rs`) |

**依存関係**: Phase 6.1（マウスモード判定。ON時はアプリ転送、OFF時に選択動作）

**リファレンス**:
- `refs/alacritty/alacritty_terminal/src/selection.rs` — Selection 型の設計（最重要）
- `refs/alacritty/alacritty/src/clipboard.rs` — クリップボードプラットフォーム抽象

**新規依存クレート**: `arboard`

---

## Phase 7: IME入力サポート

**概要**: macOS IME(日本語入力)に対応する。CJKフォント対応は済んでいるため、入力側の対応が急務。

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| winit IME イベント処理 | `WindowEvent::Ime(Preedit, Commit)` を処理し、Commit 時にPTYへ書き込み | sdit (`main.rs`) |
| IME 有効化 | `Window::set_ime_allowed(true)` + `set_ime_cursor_area()` でカーソル位置通知 | sdit (`main.rs`) |
| プリエディット表示 | 変換候補をカーソル位置にインライン描画 | sdit-render |

**依存関係**: なし（Phase 6と並行可能だが、Phase 6後を推奨）

**リファレンス**:
- `refs/alacritty/alacritty/src/input/mod.rs` — IME イベントハンドリング
- winit 公式ドキュメント(`WindowEvent::Ime`)

---

## Phase 8: フォントサイズ動的変更 + URL検出

### Phase 8.1: フォントサイズ動的変更

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| Cmd+=/- でフォントサイズ変更 | `FontContext` のサイズ変更 + メトリクス再計算 | sdit (`main.rs`) |
| アトラスのクリアと再構築 | グリフキャッシュを全クリアし、新サイズで再ラスタライズ | sdit-render (`atlas.rs`, `font.rs`) |
| Terminal リサイズ連動 | フォントサイズ変更 → セルサイズ変化 → 全セッションリサイズ | sdit (`main.rs`) |
| Cmd+0 でデフォルトサイズ復帰 | 設定ファイルのフォントサイズに復帰 | sdit (`main.rs`) |

### Phase 8.2: URL検出・クリック

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| OSC 8 ハイパーリンク対応 | Cell にハイパーリンクメタデータを保持 | sdit-core |
| URL正規表現検出 | `https?://...` パターン検出。Cmd+クリックで `open` 実行 | sdit, sdit-core |
| URL アンダーライン表示 | Cmd キー押下中のマウスホバーでURL強調 | sdit-render |

**依存関係**: Phase 6.1（マウスイベント基盤）

---

## Phase 9: 検索機能 + キーバインドカスタマイズ

### Phase 9.1: スクロールバック内検索

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| 検索UIの表示 | Cmd+F で検索バーをオーバーレイ表示 | sdit, sdit-render |
| グリッド内テキスト検索 | Grid セルを走査し、マッチする行を検出。前後マッチへジャンプ | sdit-core (新規 `search.rs`) |
| マッチのハイライト | マッチセルの背景色変更 | sdit-render |
| インクリメンタルサーチ | 入力文字ごとにリアルタイム更新 | sdit |

### Phase 9.2: キーバインドカスタマイズ

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| キーバインド設定スキーマ | TOML `[keybinds]` セクション定義 | sdit-config |
| アクション列挙型 | `NewWindow`, `NewTab`, `CloseTab`, `Copy`, `Paste`, `Search` 等をenum化 | sdit-config |
| ショートカット判定リファクタリング | `is_*_shortcut()` 関数群を設定駆動に置換 | sdit (`main.rs`) |

**依存関係**: Phase 8

**リファレンス**:
- `refs/alacritty/alacritty/src/config/bindings.rs` — キーバインド設定の型定義

---

## Phase 10: 設定Hot Reload + ウィンドウ永続化 + リガチャ/絵文字

### Phase 10.1: 設定Hot Reload

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| ファイル監視 | `notify` クレートで `sdit.toml` の変更を監視 | sdit |
| 差分適用 | フォント・カラー・キーバインドの変更をリロード | sdit, sdit-config |

**新規依存クレート**: `notify`

### Phase 10.2: ウィンドウサイズ・位置の永続化

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| ウィンドウジオメトリの保存 | クローズ時にサイズと位置を保存 | sdit-session |
| 復元 | 起動時に保存ジオメトリで生成 | sdit |

### Phase 10.3: リガチャ + カラー絵文字

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| OpenType リガチャ | cosmic-text のシェーピング結果からリガチャを検出・描画 | sdit-render |
| カラー絵文字 | COLR/CPAL テーブルからカラーグリフ抽出、Atlas にRGBA格納 | sdit-render |

**依存関係**: Phase 9.2（Hot Reload はキーバインド完成が前提）

---

## Phase 11: macOS ネイティブ統合 + GUI設定画面

### Phase 11.1: macOS メニューバー

**概要**: macOS ネイティブメニューバーを実装する。winit 0.30 にはメニューAPIがないため、`muda` クレートまたは `cocoa` 直接呼び出しで実装する。

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| muda クレート統合 | macOS ネイティブメニューバーの基盤を導入 | sdit (`main.rs`), `Cargo.toml` |
| アプリケーションメニュー | SDIT > About, Preferences, Quit | sdit |
| ファイルメニュー | New Window (Cmd+N), New Tab (Cmd+T), Close (Cmd+W) | sdit |
| 編集メニュー | Copy (Cmd+C), Paste (Cmd+V), Select All (Cmd+A) | sdit |
| 表示メニュー | Toggle Sidebar (Cmd+\\), Font Size +/- (Cmd+=/-)  | sdit |
| メニューアクションとキーバインドの統合 | `is_*_shortcut()` 関数群と Menu Action を統一的に管理 | sdit |

**依存関係**: Phase 9.2（キーバインドカスタマイズ。メニューとショートカットの一元管理が前提）

**リファレンス**:
- `muda` クレート (tauri-apps製): macOS/Windows/Linux ネイティブメニュー
- Ghostty の GTK メニュー実装: `refs/ghostty/src/apprt/gtk/class/surface.zig`

**注意**: Alacritty, WezTerm ともにメニューバーは実装していない（ショートカット駆動の設計哲学）。SDITはSDIファーストでウィンドウ単位の操作が多いため、メニューバーは操作の発見性（discoverability）向上に有効。

**新規依存クレート**: `muda`

### Phase 11.2: 右クリックコンテキストメニュー

**概要**: ターミナル領域とサイドバー領域で右クリックメニューを表示する。

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| MouseButton::Right イベント処理 | 右クリック検出（現在は Left のみ実装） | sdit (`main.rs`) |
| ターミナル領域メニュー | Copy, Paste, Select All, Search | sdit |
| サイドバー領域メニュー | Close Session, Detach to New Window, Rename | sdit |
| muda PopupMenu 統合 | ネイティブコンテキストメニューを muda で表示 | sdit |

**依存関係**: Phase 11.1（muda クレートの導入）, Phase 6.2（Copy/Paste の実装）

### Phase 11.3: GUI設定画面

**概要**: egui を wgpu パイプラインに統合し、GUI設定画面を提供する。TOML を直接編集する代替手段。

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| egui-wgpu 統合 | egui の wgpu バックエンドを既存のレンダリングパイプラインに組み込み | sdit-render, sdit |
| Preferences ウィンドウ | 別ウィンドウで設定画面を表示（Cmd+, で起動） | sdit |
| フォント設定 UI | フォントファミリー、サイズ、行高の変更 | sdit |
| カラーテーマ UI | テーマ選択（プレビュー付き） | sdit |
| キーバインド UI | キーバインドの表示・変更 | sdit |
| 設定の即時反映 | UI 変更 → TOML 書き出し → Hot Reload 連携 | sdit, sdit-config |

**依存関係**: Phase 10.1（Hot Reload）, Phase 9.2（キーバインドカスタマイズ）

**新規依存クレート**: `egui`, `egui-wgpu`

**設計方針**:
- 設定の正規データは常にTOMLファイル。GUIはTOMLの読み書きUIに過ぎない
- GUIで変更 → TOML保存 → Hot Reload で反映。二重管理を避ける
- 設定画面はターミナルウィンドウとは独立した別ウィンドウで表示

---

## Phase 12: macOS リリースビルド調査・提案

**概要**: macOS向けの本番ビルドに必要なバンドルアセットとビルド手順を**調査し、提案する**フェーズ。
実際のCI/CD構築や配布自動化は本フェーズのスコープ外とし、別フェーズで行う。

**完了条件**:
1. .app バンドルに必要なアセット一覧とその作成方法を文書化する
2. コード署名・公証に必要な手順と前提条件を文書化する
3. ビルド手順（手動）の提案書を `docs/plans/` に作成する

### Phase 12.1: バンドルアセット調査

| タスク | 詳細 |
|---|---|
| .app バンドル構造の調査 | `Contents/MacOS/`, `Contents/Resources/`, `Contents/Info.plist` の必須構成を文書化 |
| Info.plist 必須キーの調査 | CFBundleIdentifier, NSHighResolutionCapable, NSSupportsAutomaticGraphicsSwitching, LSMinimumSystemVersion 等 |
| アイコン要件の調査 | macOS アプリアイコンの必須サイズ一覧、.icns 生成手順（iconutil）、デザインガイドライン |
| entitlements 要件の調査 | wgpu/Metal 使用時に必要な entitlement、Hardened Runtime との互換性 |
| Universal Binary 要件 | x86_64 + aarch64 のクロスコンパイル方法、`lipo` による結合手順 |

**リファレンス**:
- `refs/alacritty/Makefile` — バンドル作成の参考実装
- `refs/alacritty/extra/osx/Alacritty.app/Contents/Info.plist` — Info.plist の参考
- `refs/ghostty/` — Ghostty の macOS バンドル構成

### Phase 12.2: コード署名・公証手順の調査

| タスク | 詳細 |
|---|---|
| Developer ID 証明書の確認 | Apple Developer Program で必要な証明書の種類と取得手順 |
| コード署名手順の文書化 | `codesign --options runtime` の詳細オプション、署名対象ファイルの特定 |
| 公証手順の文書化 | `xcrun notarytool submit` のワークフロー、App-specific password の設定方法 |
| DMG 作成手順の文書化 | `create-dmg` の使い方、DMG 自体の署名・公証手順 |

### Phase 12.3: ビルド手順提案書の作成

| タスク | 詳細 |
|---|---|
| 手動ビルド手順書 | 開発者がローカルで .app → 署名 → 公証 → DMG を実行できる手順書 |
| CI/CD 自動化の設計提案 | GitHub Actions での自動化に必要な Secrets、ワークフロー構成の提案（実装はしない） |
| 配布チャネルの提案 | GitHub Releases, Homebrew Cask Tap, 直接ダウンロード等の選択肢と推奨 |

**依存関係**: Phase 11（メニュー等のmacOSネイティブ機能が揃った状態でリリース準備）

**成果物**: `docs/plans/phase12-macos-release-guide.md`（調査結果と提案をまとめた文書）

---

## フェーズ依存関係

```
Phase 5.8 (クレート統合: 5クレート → 2クレート)
    ↓
Phase 5.5 (ターミナル互換性: DA/DSR/Alt/ベル/タイトル/カーソルスタイル)
    ↓
Phase 5.9 (main.rs 分割リファクタリング)
    ↓
Phase 6.1 (マウス + スクロール)
    ↓
Phase 6.2 (選択 + クリップボード)   Phase 7 (IME) ← 並行可能
    ↓
Phase 8.1 (フォントサイズ)
Phase 8.2 (URL検出) ← Phase 6.1 依存
    ↓
Phase 9.1 (検索)
Phase 9.2 (キーバインド) ← Phase 8 依存
    ↓
Phase 10.1 (Hot Reload) ← Phase 9.2 依存
    ↓
Phase 11.1 (macOSメニューバー) ← Phase 9.2 依存
Phase 11.2 (右クリックメニュー) ← Phase 11.1 + 6.2 依存
Phase 11.3 (GUI設定画面) ← Phase 10.1 + 9.2 依存
    ↓
Phase 12 (macOSリリースビルド調査) ← Phase 11 依存

─── 独立タスク（任意タイミングで着手可能）───
Phase 10.2 (ウィンドウ永続化) ← 依存なし。Phase 6 以降いつでも
Phase 10.3 (リガチャ/絵文字) ← 依存なし。Phase 6 以降いつでも
```

## 優先度の根拠

1. **Phase 5.5（最最高・即着手）**: DA/DSR/CPR がないとシェルが壊れる。Alt→ESCがないとvim/emacsが使えない。工数は極小だが影響は致命的
2. **Phase 6（最高）**: マウス報告がないとvim/htopが使えない。選択/クリップボードがないとコピー不可。日常使いの最低ライン
3. **Phase 7（高）**: 日本語入力不可ではSDITの主要ユーザーが使えない
4. **Phase 8（中高）**: フォントサイズ変更は高DPIでの快適性に直結。URL検出は開発者ワークフロー効率化
5. **Phase 9（中）**: 検索とキーバインドはパワーユーザー要望。コードのリファクタリングも兼ねる
6. **Phase 10（中低）**: ポリッシュ。Hot Reload、永続化、リガチャ等の完成度向上
7. **Phase 11（中低）**: macOSネイティブ統合。メニューバーは操作の発見性（discoverability）を向上。GUI設定画面はTOML編集の代替手段
8. **Phase 12（リリース時）**: 配布パイプライン。Apple Developer Program 加入済みのため、コード署名・公証・DMG配布を自動化

---

## セキュリティ考慮事項

- **Phase 5.5**: DA応答のバージョン文字列にセキュリティ上有用な情報を含めない。CPR応答の座標値を境界内にクランプ
- **Phase 6**: マウス座標の境界チェック。悪意あるアプリがマウスモードを意図的にONにしてユーザー操作を妨害するリスク
- **Phase 6.2**: OSC 52 クリップボード操作は要注意。悪意あるエスケープシーケンスでクリップボードを書き換えるリスクあり → ユーザー確認ダイアログまたは設定で無効化可能にする
- **Phase 8.2**: URL検出で`open`コマンド実行。悪意あるURLの自動実行リスク → ユーザーが明示的にCmd+クリックした場合のみ
- **Phase 10.1**: 設定ファイル監視。TOCTOU競合のリスク → アトミック読み込みで対応
