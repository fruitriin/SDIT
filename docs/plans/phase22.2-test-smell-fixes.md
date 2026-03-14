# Phase 22.2: テストスメル修正（savanna-smell-detector 妥当指摘分）

## 背景

savanna-smell-detector v0.1.0 で全 severity レベルのスキャンを実施（397件検出）。
実際のテストコードを1件ずつ確認し、妥当な指摘 ~72件を特定した。
本計画ではこれらのうち修正可能なものを対応する。

- 検出結果の詳細: https://github.com/fruitriin/savanna-smell-detector/issues/11
- 対象コミット: `197f4ab`

## 目標

- CI ゲート（`--min-severity 4 --fail-on-smell`）をクリーンに通過する状態を維持
- severity 3 の妥当な指摘を可能な限り解消する
- severity 1-2 の低コスト修正を実施する

## スコープ外

- Assertion Roulette（273件）— 93%が過剰検出。ツール側の改善待ち
- Conditional Test Logic のテーブル駆動テスト / 状態構築ループ / 全要素走査（~30件）— 過剰検出
- Ignored Test（2件）— 理由付き `#[ignore = "..."]` であり正当
- Sleepy Test の PTY/GUI 統合テスト（4件）— 実プロセス待機でありやむを得ない

---

## サブタスク

### ST-1: Missing Assertion 修正（severity 4, 1件）

**対象:**
- `crates/sdit-core/src/config/color.rs:440` — `from_theme_all_variants`

**現状:**
```rust
#[test]
fn from_theme_all_variants() {
    for theme in ThemeName::all() {
        let _ = ResolvedColors::from_theme(theme);
    }
}
```

**修正方針:**
- `let _ =` を `let colors =` に変更し、基本的なアサーションを追加する
- 例: `assert_ne!(colors.background, colors.foreground, "theme {theme:?}: bg == fg")` 等
- 「全テーマがパニックせずにパースでき、最低限の品質を持つ」ことを表明する

---

### ST-2: Sleepy Test 修正（severity 3, 3件）

時刻注入パターンで `sleep()` を除去する。

**対象:**
1. `crates/sdit/src/app.rs:1040` — `visual_bell_fades_to_zero`
2. `crates/sdit/src/app.rs:1048` — `visual_bell_completed_clears_state`
3. `crates/sdit/src/quick_terminal.rs:441` — `quick_terminal_state_finish_slide_out`

**修正方針:**

**(a) VisualBell（2件）**
- `VisualBell` に「現在時刻を外部から注入できるメソッド」を追加する
  - 例: `intensity_at(now: Instant) -> f32`, `completed_at(now: Instant) -> bool`
  - または `ring()` 時の `Instant` を記録し、テスト側で `ring_at(start: Instant)` + `intensity_at(now: Instant)` とする
- テスト側で `Instant::now()` + `Duration::from_millis(20)` を足した未来時刻を渡す
- `sleep()` を除去

**(b) QuickTerminalAnimation（1件）**
- `finish_animation()` が内部で `Instant::now()` を使ってアニメーション完了を判定しているはず
- `finish_animation_at(now: Instant)` を追加するか、`start_time` を過去に設定するテストヘルパーを用意する
- `sleep()` を除去

**変更影響範囲:**
- `VisualBell` / `QuickTerminalState` の公開 API に `_at(Instant)` メソッドが追加される
- 既存の `intensity()` / `completed()` / `finish_animation()` は `Instant::now()` を使う薄いラッパーとして残す
- プロダクションコードへの影響は最小限

---

### ST-3: Redundant Print 除去（severity 1, 10件）

**対象と修正方針:**

**(a) `is_tty()` ガード + `eprintln!` パターン（7件）**

場所:
- `crates/sdit-core/src/pty/mod.rs:218` — `test_pty_spawn_and_echo`
- `crates/sdit-core/src/pty/mod.rs:260` — `test_pty_spawn_shell`
- `crates/sdit-core/src/pty/mod.rs:278` — `test_pty_resize`
- `crates/sdit-core/src/pty/mod.rs:299` — `test_pty_write_and_read`
- `crates/sdit-core/tests/headless_pipeline.rs:63` — `echo_appears_in_grid`
- `crates/sdit-core/tests/headless_pipeline.rs:96` — `shell_command_pipeline`
- `crates/sdit-core/tests/headless_pipeline.rs:141` — `pty_spawn_then_resize`
- `crates/sdit-core/tests/headless_pipeline.rs:171` — `cursor_position_after_escape_sequence`

修正: `eprintln!("skipping...")` を除去し、`return` のみにする。
理由: テスト出力に混じるスキップメッセージは `cargo test` の出力を汚す。テストフレームワークが `-- --nocapture` 時のみ表示するため、明示的な `eprintln!` は不要。

**(b) GUI テストの診断出力（2件）**

場所:
- `crates/sdit/tests/gui_interaction.rs:112` — `window_appears_and_captures_screenshot`
- `crates/sdit/tests/source_file_limits.rs:49` — `source_files_within_line_limits`

修正: `eprintln!` を除去するか、`log::info!` に置換する（テスト用 `RUST_LOG` で制御可能にする）。

**(c) Conditional Test Logic（環境依存スキップ）の同時改善**

ST-3(a) の `is_tty()` + return パターンは Conditional Test Logic（severity 3）としても検出されている。
`eprintln!` を除去するだけでは Conditional Test Logic は解消しないが、本 Phase のスコープでは `eprintln!` 除去のみとし、
`is_tty()` ガードの `#[ignore]` 化は将来の Phase に委ねる。

理由: `#[ignore]` 化すると CI で明示的に `--ignored` を渡さないと実行されなくなる。
現在の `is_tty()` ガードは「TTY 環境なら自動実行、非 TTY なら静かにスキップ」という振る舞いであり、
これを変更するには CI 設定の見直しも必要。

---

### ST-4: Magic Number 定数化（severity 2, 妥当分）

**対象と修正方針:**

**(a) コマンドパレット入力上限 `256`**

場所: `crates/sdit/src/command_palette.rs:37`

修正:
```rust
const MAX_INPUT_BYTES: usize = 256;
```
を定義し、`push_str()` とテスト `push_str_accepts_exactly_256_bytes` で参照する。

**(b) クイックセレクトラベル数 `52`**

場所: `crates/sdit/src/app.rs:1183` — `hint_labels_are_unique`

修正:
```rust
// QuickSelectState に定数を追加、またはテスト内で計算
let expected_count = QuickSelectState::CHARS.len() * 2; // 26 single + 26 two-char (aa-az)
```
テスト内で `52` を直接書く代わりに、`CHARS` の長さから導出する。
`CHARS` が `pub(crate)` でない場合はアクセスレベルを調整する。

**(c) フォールバック解像度 `800x600`**

場所: `crates/sdit/src/quick_terminal.rs:323-324`

修正:
```rust
const FALLBACK_SCREEN_WIDTH: u32 = 800;
const FALLBACK_SCREEN_HEIGHT: u32 = 600;
```
を定義し、プロダクションコードとテストの両方で参照する。

**(d) テスト解像度 `1920x1080`, `2560x1440`**

場所: `crates/sdit/src/quick_terminal.rs` テスト群

修正: テスト内にローカル定数として定義:
```rust
const TEST_FHD: (u32, u32) = (1920, 1080);
const TEST_QHD: (u32, u32) = (2560, 1440);
```

---

## 実装順序

1. **ST-1** → ST-4(a)(b) → ST-2 → ST-3 → ST-4(c)(d)
2. ST-1 は severity 4 で CI ブロッカーになりうるため最優先
3. ST-2 は VisualBell / QuickTerminalState の API 変更を伴うため、単体で実施
4. ST-3 と ST-4 は機械的な変更であり並列実施可能

## 依存クレート

新規依存なし。

## セキュリティ考慮

テストコードのみの変更であり、プロダクションコードへの影響は ST-2 の `_at(Instant)` メソッド追加のみ。
セキュリティリスクなし。

## 完了条件

- `savanna-smell-detector --min-severity 4 --fail-on-smell crates/` がゼロ検出で通過
- `savanna-smell-detector --min-severity 3 crates/` で Sleepy Test が 4件以下（PTY/GUI 統合テストのみ）
- `savanna-smell-detector --min-severity 1 crates/` で Redundant Print が 0件
- `cargo fmt --check && cargo clippy --all-targets && cargo test` が通過
