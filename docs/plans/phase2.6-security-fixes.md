# Phase 2.6 — テストユーティリティ セキュリティ修正

Phase 2.5 セキュリティレビューで先送りされた Medium 2件を修正する。

## 出典

`docs/plans/phase2.5-integration-testing.md` セキュリティレビュー結果より:

| ID | 重要度 | 内容 |
|---|---|---|
| M-1 | Medium | `capture-window.swift` の出力パスにバリデーションがない（パストラバーサル） |
| M-3 | Medium | `send-keys.sh` / `capture-window.swift` のプロセス名一致が basename 比較のみ（なりすまし可能） |

## タスク

### M-1: capture-window.swift 出力パスバリデーション

- [ ] `outputPath` がプロジェクトルート配下（特に `tmp/`）であることを検証する
- [ ] 絶対パスに正規化後、許可リスト（ワーキングディレクトリ配下）に含まれるかチェック
- [ ] 違反時は exit 1 + stderr にメッセージ

**修正方針:**
```swift
let resolved = URL(fileURLWithPath: outputPath).standardized.path
let cwd = FileManager.default.currentDirectoryPath
guard resolved.hasPrefix(cwd) else {
    fputs("Error: output path must be under working directory\n", stderr)
    exit(1)
}
```

### M-3: プロセス特定の強化

- [ ] `send-keys.sh`: `pgrep -x` で PID を取得し、AppleScript で PID ベースの指定に変更
- [ ] `capture-window.swift`: 現状 PID ベースで動作しているが、`findPid` の basename 比較を厳格化（フルパス一致 or PID 直接指定オプション追加）

**send-keys.sh 修正方針:**
```bash
PID=$(pgrep -x "$PROCESS_NAME" | head -1)
# AppleScript で PID ベースのプロセス指定
osascript <<APPLESCRIPT
tell application "System Events"
    set targetApp to first process whose unix id is $PID
    ...
end tell
APPLESCRIPT
```

## 対象ファイル

- `tools/test-utils/capture-window.swift`
- `tools/test-utils/send-keys.sh`
- `tools/test-utils/CLAUDE.md`（セキュリティ注意書き更新）

## 完了条件

- M-1, M-3 とも修正完了
- `tools/test-utils/build.sh` でビルド成功
- `phase2.5-integration-testing.md` のセキュリティレビュー表を更新
