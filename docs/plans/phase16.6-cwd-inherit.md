# Phase 16.6: Working Directory 継承

**概要**: 新しいウィンドウ/セッションを開くとき、直前のセッションのカレントディレクトリを引き継ぐ。

**状態**: **完了**

## タスク

| タスク | 詳細 | 変更先 | 状態 |
|---|---|---|---|
| WindowConfig に inherit_working_directory 追加 | デフォルト true | sdit-core (`config/mod.rs`) | **完了** |
| OSC 7 で取得した CWD を Session に保持 | Terminal の CWD 更新を Session に伝搬 | sdit-core (`terminal/handler.rs`, `session/`) | **完了** |
| 新セッション生成時に CWD を渡す | アクティブセッションの CWD を SpawnParams に設定 | sdit (`window_ops.rs`) | **完了** |
| テスト | 設定デシリアライズ | sdit-core | **完了** |

## 設定例

```toml
[window]
inherit_working_directory = true
```

## 参照

- `refs/ghostty/src/config/Config.zig` — window-inherit-working-directory
