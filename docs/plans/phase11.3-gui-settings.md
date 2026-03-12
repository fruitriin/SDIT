# Phase 11.3: GUI設定画面

**概要**: egui を wgpu パイプラインに統合し、GUI設定画面を提供する。TOML を直接編集する代替手段。

## タスク

| タスク | 詳細 | 変更先クレート |
|---|---|---|
| egui-wgpu 統合 | egui の wgpu バックエンドを既存のレンダリングパイプラインに組み込み | sdit-render, sdit |
| Preferences ウィンドウ | 別ウィンドウで設定画面を表示（Cmd+, で起動） | sdit |
| フォント設定 UI | フォントファミリー、サイズ、行高の変更 | sdit |
| カラーテーマ UI | テーマ選択（プレビュー付き） | sdit |
| キーバインド UI | キーバインドの表示・変更 | sdit |
| 設定の即時反映 | UI 変更 → TOML 書き出し → Hot Reload 連携 | sdit, sdit-config |

## 依存関係

- Phase 10.1（Hot Reload）
- Phase 9.2（キーバインドカスタマイズ）

## 新規依存クレート

`egui`, `egui-wgpu`

## 設計方針

- 設定の正規データは常にTOMLファイル。GUIはTOMLの読み書きUIに過ぎない
- GUIで変更 → TOML保存 → Hot Reload で反映。二重管理を避ける
- 設定画面はターミナルウィンドウとは独立した別ウィンドウで表示
