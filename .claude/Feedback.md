# Process Feedback

開発プロセスの振り返りと改善を記録する。

## 記録方法

タスク完了時や問題発生時に、以下のいずれかのセクションに追記する。

## オーナーフィードバック

## 問題の記録

- ビルド破壊の未コミット残留
前セッションのエージェントが `session/persistence.rs`（SessionRestoreInfo, WindowSnapshot, window_sessions）と `config/keybinds.rs`（ToggleCommandPalette, all_with_names）および `command_palette.rs` を作成・変更したが、これらをコミットせずにセッションを終了した。一方でこれらを参照する `event_loop.rs` や `window_ops.rs` の変更はコミットされており、main ブランチが壊れた状態になっていた。

- アプリケーションのアイコンがないとクラッシュする件は Phase 21.6 で根本修正済み（dangling ポインタ起因）。
  app アイコンなしでもメニュークリックでクラッシュしないことを確認。ただし macOS Dock 表示の見栄えのためダミー PNG を作成することは引き続き推奨。

## 改善アクション

- `.claude/settings.local.json` のフックコマンドは**絶対パス**で書くこと。相対パス `python3 .claude/hooks/check-tmp.py` は、Claude Code セッション内で `cd tools/test-utils/` 等のサブディレクトリ移動が起きると解決できなくなり、Bash/Edit/Write がすべてブロックされる。修正済み（絶対パスに変更）。

- `list-menus.sh` の `joinList` AppleScript ハンドラが `tell application "System Events"` ブロック内で呼び出すと機能せず、空のメニューリストを返す問題がある。手動ループに置き換えることで修正可能（Phase 21.5 統合テストで発見）

- `annotate-grid` の画像反転バグ（Phase 24 統合テストで発見、2026-03-14）:
  `screencapture` フォールバック経由のPNGを入力すると、出力画像の元画像コンテンツが上下反転する。
  根本原因: `screencapture` 生成PNGは既に正立（上が上）だが、`annotate-grid.swift` のL350-352の
  `translateBy(x:0, y:H) + scaleBy(x:1, y:-1)` flip変換が余分に適用されている。
  ラベル座標（0,0が左上）は正しいが、元画像コンテンツが逆さまになるため LLM の判定に混乱を招く。
  修正案: CGImageの向きを確認して条件分岐するか、flip変換を削除してCGContextのY軸を直接扱う。
  影響範囲: `tools/test-utils/annotate-grid.swift` のみ。シナリオ 029 に既知の注意事項として記録済み。

## 完了済み

- 依存パッケージ提案ルール → CLAUDE.md 依存クレート方針に反映（Plan 段階でクレート名・用途・代替手段を明記）
- セキュリティ/統合テストのフィードバック修正を実装エージェントに移譲 → Progress テンプレート Stage 2 制御フローに反映
- 統合テストをリグレッションとして最初に実行 → Progress テンプレート Stage 1 にリグレッション優先ルール追加
- 統合テストの適応的実行モード（安定→投機的、不安定→ステップ） → Progress テンプレート integration-test に反映
- 関心事の異なるバグは新プランに分離 → Progress テンプレート フィードバック集約にバグ分離ルール追加
- CJK テスト右端輝度分析 → `docs/knowhow/gui-test-cjk-validation.md` に記録
- E2E テスト補助ツール提案義務 → Progress テンプレート integration-test に反映
- 和文テキスト対照群 → `docs/knowhow/gui-test-cjk-validation.md` に検討結果を記録
- 計画ファイルの関心事別分離 → CLAUDE.md ブートシーケンスの Plan 作成ルールに追記
- integration-test エージェントが `subagent_type: Explore`（読み取り専用）で起動されていたため GUI テスト（SDIT 起動 + スクリーンショット）を実行できていなかった → テンプレート・CLAUDE.md を修正し `general-purpose` を使うよう明記
- worktree 起動時に `.claude` ディレクトリ（hooks 等の .gitignore 対象ファイル含む）が複製されず、フックエラーでエージェントがブロックされていた → CLAUDE.md に `cp -r .claude <worktree>/.claude` ルールを追記
- ノウハウの読み込み順のブラッシュアップ → CLAUDE.md ブートシーケンスを knowhow サブエージェントフィルタリング方式に変更
- 統合テストのログチェック → `smoke_headless.rs` と `smoke_gui.rs` に `RUST_LOG=info` + 期待ログメッセージの存在確認を追加
- Phase 2 セキュリティ Low は各 Plan に記録済み（独立計画不要）
- Phase 2.5 セキュリティ Low L-1〜L-4 は `docs/plans/phase2.5-integration-testing.md` に記録済み
- Phase 2.6 セキュリティ Low L-1〜L-3 / Info I-1〜I-3 は `docs/plans/phase2.6-security-fixes.md` に記録済み
- Phase 4 セキュリティ Medium M-1〜M-3 修正済み、Low/Info は `docs/plans/phase4-session-sidebar.md` に記録済み
- Phase 5 オーナーフィードバック（日本語フォント・カラーコントラスト・統合テスト）→ Phase 5.2/5.3/5.5 で対応済み
- Phase 5 セキュリティ Medium M-1〜M-3 修正済み、Low/Info は `docs/plans/phase5-config-polish.md` に記録済み
- worktree 内フックエラー + 残存 worktree ブロック → CLAUDE.md に `.claude` コピールール追記済み、worktree クリーンアップ実施済み
