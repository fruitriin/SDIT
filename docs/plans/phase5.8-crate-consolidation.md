# Phase 5.8: クレート統合リファクタリング

## 背景

現在5クレート・6740行。平均1350行/クレートで過分割状態。

```
現状:
  sdit (binary)     1292行  → main.rs 1ファイルに全部入り
  sdit-core (lib)   2731行  → 適正（PTY/VTE/Grid）
  sdit-render (lib) 1057行  → 3ファイルしかない
  sdit-session (lib) 533行  → 4ファイル、最大166行
  sdit-config (lib)  418行  → 3ファイル、最大219行
```

クレート境界のコスト:
- Cargo.toml の依存管理 × 5
- クレート間の型変換・再エクスポート
- `use sdit_*::` のインポート散乱
- 変更時のクレート間ビルド依存

sdit-session(533行) と sdit-config(418行) はクレートにする規模ではない。
sdit-render(1057行) は sdit-core に吸収しても grid/terminal と同程度。

## 統合方針

```
統合後:
  sdit (binary)     → main.rs 分割は Phase 5.9 で対応
  sdit-core (lib)   → PTY/VTE/Grid + render + session + config を全て含む
```

### なぜ sdit-core に統合するか

- sdit-core は「GUIに依存しないライブラリ」という位置づけ
- render は wgpu に依存するが、wgpu 自体は GUI フレームワーク(winit)に依存しない
- session は PTY/Terminal に密結合しており、分離の利点が薄い
- config は全クレートが参照するため、core に置くのが自然
- 将来クレートが大きくなったら再分割すればよい（その判断材料が今はある）

### 依存グラフの変化

```
Before:
  sdit → sdit-core, sdit-render, sdit-session, sdit-config
  sdit-render → sdit-core, sdit-config
  sdit-session → sdit-core

After:
  sdit → sdit-core
```

## タスク

### Step 1: sdit-config → sdit-core に統合

| タスク | 詳細 | 工数 |
|---|---|---|
| ファイル移動 | `config/src/{lib,color,font}.rs` → `sdit-core/src/config/` | 極小 |
| モジュール宣言追加 | `sdit-core/src/lib.rs` に `pub mod config;` | 極小 |
| 依存移動 | `serde`, `toml`, `dirs` を sdit-core の Cargo.toml に移動 | 極小 |
| インポート書き換え | `use sdit_config::` → `use sdit_core::config::` (全クレート) | 小 |
| sdit-config クレート削除 | `crates/sdit-config/` 削除、ワークスペース Cargo.toml から除外 | 極小 |
| テスト通過確認 | config の19テストがそのまま通ることを確認 | 極小 |

### Step 2: sdit-session → sdit-core に統合 ✅ 完了

| タスク | 詳細 | 工数 |
|---|---|---|
| ファイル移動 | `session/src/{session,sidebar,window_registry,persistence}.rs` → `sdit-core/src/session/` | 極小 |
| モジュール宣言追加 | `sdit-core/src/lib.rs` に `pub mod session;` | 極小 |
| 依存移動 | `rustix` を sdit-core の Cargo.toml に追加 | 極小 |
| インポート書き換え | `use sdit_session::` → `use sdit_core::session::` (main.rs) | 小 |
| sdit-session クレート削除 | `crates/sdit-session/` 削除、ワークスペース Cargo.toml から除外 | 極小 |
| テスト通過確認 | session の16テストが全て通過（99テスト中に含む） | 極小 |

### Step 3: sdit-render → sdit-core に統合

| タスク | 詳細 | 工数 |
|---|---|---|
| ファイル移動 | `render/src/{pipeline,atlas,font}.rs` → `sdit-core/src/render/` | 極小 |
| モジュール宣言追加 | `sdit-core/src/lib.rs` に `pub mod render;` | 極小 |
| 依存移動 | `wgpu`, `cosmic-text`, `bytemuck` を sdit-core に移動 | 小 |
| インポート書き換え | `use sdit_render::` → `use sdit_core::render::` | 小 |
| 内部参照の簡略化 | `use sdit_core::grid::` → `use crate::grid::` (render内) | 小 |
| sdit-render クレート削除 | `crates/sdit-render/` 削除 | 極小 |
| テスト通過確認 | render の6テストがそのまま通ることを確認 | 極小 |

### Step 4: 最終確認

| タスク | 詳細 | 工数 |
|---|---|---|
| ワークスペース Cargo.toml 整理 | members から削除したクレートを除外 | 極小 |
| `cargo fmt --check && cargo clippy --all-targets` | 全体の lint 通過 | 極小 |
| `cargo test` | 全112テスト通過確認 | 極小 |
| CLAUDE.md のクレート構成セクション更新 | 新しい構成を反映 | 小 |
| docs/knowhow 更新 | architecture-decisions.md に統合理由を記録 | 小 |

## 統合後の sdit-core 構造

```
crates/sdit-core/src/
├── lib.rs
├── index.rs
├── config/          ← sdit-config から統合
│   ├── mod.rs
│   ├── color.rs
│   └── font.rs
├── grid/
│   ├── mod.rs
│   ├── cell.rs
│   ├── row.rs
│   └── storage.rs
├── terminal/
│   ├── mod.rs
│   └── handler.rs
├── pty/
│   └── mod.rs
├── font/
│   └── mod.rs
├── render/          ← sdit-render から統合
│   ├── mod.rs
│   ├── pipeline.rs
│   ├── atlas.rs
│   └── font.rs
└── session/         ← sdit-session から統合
    ├── mod.rs
    ├── session.rs
    ├── sidebar.rs
    ├── window_registry.rs
    └── persistence.rs
```

推定行数: ~4740行（現 sdit-core 2731 + render 1057 + session 533 + config 418）

## 完了条件

- [x] クレートが2つ（sdit, sdit-core）になっている
- [x] 全105ユニットテスト + 統合テスト通過
- [x] `cargo clippy --all-targets` 警告なし
- [x] 機能変更なし（純粋なリファクタリング）
- [x] CLAUDE.md のクレート構成が更新されている

## 実装結果（2026-03-12）

Step 1〜3 を順次実行し、全クレートを sdit-core に統合完了。
- sdit-config → sdit-core/src/config/
- sdit-session → sdit-core/src/session/
- sdit-render → sdit-core/src/render/
依存グラフは `sdit → sdit-core` のみに簡素化。

## 依存関係

- 前提: なし（現時点で即着手可能）
- 後続: Phase 5.5（ターミナル互換性）、Phase 5.9（main.rs 分割）

## 再分割の判断基準

統合後、以下の条件を満たしたモジュールはクレートに再昇格する:
- 単一モジュールが 1500行 を超えた
- 独立したコンパイル・テストサイクルが必要になった
- 外部クレートとして公開する理由ができた（例: sdit-core を CLI ツールから使う）
