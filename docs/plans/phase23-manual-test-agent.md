# Phase 23: マニュアル検証エージェント（/manual-test スキル）

## 概要

`docs/manuals/` に記載された操作をエージェントが実際に実行し、
動作確認・不整合レポートを生成するスキルを作成する。

毎フェーズの統合テストとは独立して、オーナーが任意のタイミングで呼び出せる。

## 動機

- マニュアルと実装の乖離（ドキュメントドリフト）を定期的に検出する
- リリース前・大きな機能追加後のリグレッション確認に使う
- ユーザー視点（マニュアルを読んだ人が操作する順序）でのテストカバレッジを確保する

## 実行タイミング（想定）

- macOS リリース前（`phase12-macos-release.md` 系フェーズ）
- マニュアルに影響する大きな機能追加後
- オーナーが手動で `/manual-test` を呼び出したとき

## スキル仕様

### 呼び出し方

```
/manual-test
/manual-test keybinds       # 特定マニュアルだけ検証
/manual-test basic-usage
```

### エージェントの動作フロー

1. **マニュアル解析**: `docs/manuals/` を全読みし、検証可能な操作を抽出する
2. **分類**: 各操作を以下に分類する
   - 🟢 自動検証可能: キーバインド、UI 表示、テキスト出力など
   - 🟡 部分検証可能: 設定オプション（一部のみ検証）
   - 🔴 自動検証不可: ドラッグ＆ドロップ、ホバー演出など
3. **SDIT 起動**: バイナリを起動し、初期状態を確認する
4. **順次検証**: 自動検証可能な操作を `docs/manuals/` の記述順に実行する
   - `tools/test-utils/send-keys.sh` でキー入力
   - `tools/test-utils/capture-window.swift` でスクリーンショット
   - `tools/test-utils/verify-text.swift` で OCR 検証
5. **レポート生成**: 結果を `tmp/manual-test-YYYY-MM-DD.md` に出力する

### レポート形式

```markdown
# マニュアル検証レポート — YYYY-MM-DD

## 結果サマリー
- ✅ 確認済み: N 件
- ❌ 失敗: N 件
- ⏭ スキップ（自動検証不可）: N 件

## 詳細

### basic-usage.md

| 操作 | キー | 結果 | 備考 |
|---|---|---|---|
| 新しいウィンドウ | Cmd+N | ✅ | |
| 検索を開く | Cmd+F | ✅ | |
| ... | | | |

### keybinds.md

...

## 不整合・要確認事項

- ❌ `Cmd+Shift+N` でセッション切り出し → ウィンドウが生成されなかった
- ...
```

## 検証対象マニュアル

| マニュアル | 検証可能な主な操作 |
|---|---|
| `basic-usage.md` | ウィンドウ操作、セッション追加/切り替え、検索、フォントサイズ変更 |
| `keybinds.md` | 全キーバインドの動作確認 |
| `themes.md` | テーマ切り替え（NextTheme/PreviousTheme） |
| `configuration.md` | 主要設定オプションの適用確認（一部） |
| `installation.md` | 検証対象外（手順書のため） |

## 自動検証不可な操作（スキップ対象）

- ドラッグ＆ドロップによるタブ切り出し・合体
- URL の Cmd+クリックでブラウザ起動
- IME 候補ウィンドウの表示位置
- マウスホバーによる URL 下線表示

## スキル実装

`~/.claude/skills/manual-test.md` として作成する。
`tools/test-utils/` の既存ツールを活用する。

## 完了条件

- [ ] `/manual-test` スキルが動作する
- [ ] `docs/manuals/basic-usage.md` と `docs/manuals/keybinds.md` の主要操作を検証できる
- [ ] レポートが `tmp/manual-test-YYYY-MM-DD.md` に生成される
- [ ] スキップ対象の操作が明示される

## セキュリティレビュー

外部入力なし・読み取り専用の検証エージェントであるため、セキュリティリスクは低い。

## 実装メモ

- スキルファイルの作成方法は `/skill-creator` を参考にする
- GUI テスト補助ツールの詳細は `tools/test-utils/README.md` を参照
