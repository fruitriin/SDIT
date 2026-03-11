# Phase 2.6 — テストユーティリティ セキュリティ修正

Phase 2.5 セキュリティレビューで先送りされた Medium 2件を修正する。

## 出典

`docs/plans/phase2.5-integration-testing.md` セキュリティレビュー結果より:

| ID | 重要度 | 内容 |
|---|---|---|
| M-1 | Medium | `capture-window.swift` の出力パスにバリデーションがない（パストラバーサル） |
| M-3 | Medium | `send-keys.sh` / `capture-window.swift` のプロセス名一致が basename 比較のみ（なりすまし可能） |

## タスク

### M-1: capture-window.swift 出力パスバリデーション — **完了**

- [x] `outputPath` がプロジェクトルート配下（特に `tmp/`）であることを検証する
- [x] 絶対パスに正規化後、許可リスト（ワーキングディレクトリ配下）に含まれるかチェック
- [x] 違反時は exit 1 + stderr にメッセージ

**実装:** `URL(fileURLWithPath:).standardized.path` で正規化後、`cwd + "/"` の hasPrefix チェック。

### M-3: プロセス特定の強化 — **完了**

- [x] `send-keys.sh`: `pgrep -x` で PID を取得し、AppleScript で PID ベースの指定に変更
- [x] `capture-window.swift`: `findPid` をフルパス優先+basename フォールバック（警告付き）に変更、`--pid` オプション追加

### M-NEW-1: send-keys.sh プロセス名バリデーション — **完了**

Phase 2.6 セキュリティレビューで新たに検出。`pgrep -x` に正規表現メタキャラクタが渡される問題。

- [x] `PROCESS_NAME` を `^[a-zA-Z0-9._-]+$` に制限するバリデーションを追加

## Phase 2.6 セキュリティレビュー結果

| ID | 重要度 | 対象 | 内容 | 対応 |
|---|---|---|---|---|
| M-NEW-1 | Medium | send-keys.sh | `pgrep -x` に正規表現メタキャラクタが渡せる | **修正済み** |
| L-1 | Low | capture-window.swift | `standardized` はシンボリックリンクを解決しない | 記録のみ（攻撃者の事前配置が必要、影響限定的） |
| L-2 | Low | send-keys.sh | 同名プロセス複数時の非決定的 PID 選択 | 記録のみ（テストツールとして許容） |
| L-3 | Low | capture-window.swift | basename フォールバックが警告付きで有効 | 記録のみ（後方互換、警告で検出可能） |
| I-1 | Info | capture-window.swift | `String` の単純連結による末尾スラッシュ依存 | 記録のみ |
| I-2 | Info | send-keys.sh | `$PID` 展開は問題ないが設計上のノイズ | 記録のみ |
| I-3 | Info | capture-window.swift | `--pid` に PID 範囲チェックなし | 記録のみ |

## 対象ファイル

- `tools/test-utils/capture-window.swift`
- `tools/test-utils/send-keys.sh`
- `tools/test-utils/CLAUDE.md`（セキュリティ注意書き更新）

## 完了条件

- [x] M-1, M-3 とも修正完了
- [x] M-NEW-1 修正完了
- [x] `tools/test-utils/build.sh` でビルド成功（警告なし）
- [x] `phase2.5-integration-testing.md` のセキュリティレビュー表を更新
