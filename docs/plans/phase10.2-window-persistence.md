# Phase 10.2: ウィンドウサイズ・位置の永続化

**状態**: **完了** (2026-03-13)

**概要**: ウィンドウのサイズと位置を保存し、次回起動時に復元する。

## タスク

| タスク | 詳細 | 変更先クレート | 状態 |
|---|---|---|---|
| ウィンドウジオメトリの保存 | クローズ時にサイズと位置を保存 | sdit-core, sdit | 完了 |
| 復元 | 起動時に保存ジオメトリで生成 | sdit | 完了 |

## 実装内容

- `WindowGeometry` 型: width/height (f64, logical) + x/y (i32, physical)
- `AppSnapshot` に `windows: Vec<WindowGeometry>` 追加（`#[serde(default)]` で後方互換）
- `close_window()` で残存ウィンドウのジオメトリを `session.toml` に保存
- `resumed()` で `AppSnapshot::load()` → `create_window(geometry)` で復元
- `validated()` メソッドで不正値（NaN/Inf/極小/極大/オフスクリーン）をクランプ

## 依存関係

なし（独立・任意タイミングで着手可能。Phase 6 以降いつでも）

## セキュリティレビュー結果

### 修正済み

- **M-1 (Medium)**: WindowGeometry 復元時バリデーション欠如 → `validated()` メソッドで NaN/Inf/極値をクランプ

### 記録（Low/Info — 将来対応）

- **L-1 (Low)**: windows リストの要素数上限なし — 現実的には問題にならない
- **L-2 (Low)**: WindowGeometry に Default 実装なし — validated() があるため実害なし
- **I-1 (Info)**: 物理座標と論理座標の混在 — 意図的な設計、ドキュメントコメントで明記済み
- **I-2 (Info)**: collect_session_snapshots() が常に空 — 将来のセッション復元フェーズで対応
