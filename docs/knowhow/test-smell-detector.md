# savanna-smell-detector 導入ノウハウ

## 概要

savanna-smell-detector はRustテストコードのスメル（品質問題）を検出するツール。
SDIT では severity 1（最厳格）で CI ゲートに組み込み済み。

## CI 設定

```bash
# scripts/check.sh
SMELL_MAGIC_WHITELIST="24,80,0,1,255,256,4096"
savanna-smell-detector --min-severity 1 --fail-on-smell \
  --magic-number-whitelist "$SMELL_MAGIC_WHITELIST" \
  --assertion-roulette-threshold 5 crates/
```

### オプションの意味

- `--magic-number-whitelist "24,80,0,1,255,256,4096"`: ターミナル業界の標準値（24行80桁）、境界値（0,1）、8bit境界（255,256）、バッファ上限（4096）を除外
- `--assertion-roulette-threshold 5`: メッセージなし assert が5個以上あるテストのみ検出。Rust は `file:line` がパニック時に表示されるため、少数なら問題にならない

## severity 段階的引き下げ戦略

1. まず severity 4 で CI に入れる（ほぼノイズなし）
2. 検出されたスメルを修正 or smell-allow でサプレス
3. severity を1段下げて繰り返す
4. 最終的に severity 1 まで到達

## smell-allow パターン集

テスト関数の直前にコメントで記述。複数タイプはカンマ区切り。

```rust
// smell-allow: conditional-test-logic, silent-skip — TTY がない CI では PTY テスト不可
#[test]
fn test_pty_spawn() { ... }

// smell-allow: redundant-print — #[ignore] 手動実行テスト。eprintln! は診断出力として意図的
#[test]
#[ignore = "GUI 環境が必要"]
fn gui_test() { ... }

// smell-allow: fragile-test — sleep ではなく時刻算術で未来の時点を生成
#[test]
fn visual_bell_fades() { ... }

// smell-allow: magic-number — ピクセルデータは連番パターンで意図が明確
#[test]
fn write_stores_rgba_pixels() { ... }
```

## よく使う修正パターン

### 1. Config デフォルト値の定数化

```rust
// Before: マジックナンバー
assert_eq!(bell.duration_ms, 150);

// After: 定数参照
impl BellConfig {
    pub const DEFAULT_DURATION_MS: u32 = 150;
}
assert_eq!(bell.duration_ms, BellConfig::DEFAULT_DURATION_MS);
```

### 2. デシリアライズテストの入力変数バインド

```rust
// Before: 入力と期待値が分離
let toml_str = "[bell]\nduration_ms = 200\n";
let config: Config = toml::from_str(&toml_str).unwrap();
assert_eq!(config.bell.duration_ms, 200);

// After: 入力変数を assert でも参照（t_wada 流）
let input_duration = 200;
let toml_str = format!("[bell]\nduration_ms = {input_duration}\n");
let config: Config = toml::from_str(&toml_str).unwrap();
assert_eq!(config.bell.duration_ms, input_duration);
```

### 3. グリッドテストの意図的変数命名

```rust
// Before
let mut grid = Grid::new(5, 10, 0);
grid.resize(8, 10);

// After
let initial_rows = 5;
let taller_rows = 8;
let mut grid = Grid::new(initial_rows, 10, 0);
grid.resize(taller_rows, 10);
```

### 4. 環境依存テストの分離

```rust
// Before: 1テストに環境分岐
#[test]
fn test_default_shell() {
    if let Ok(shell) = std::env::var("SHELL") {
        assert_eq!(config.shell, Some(shell));
    } else {
        assert_eq!(config.shell, None);
    }
}

// After: 2テストに分離
#[test]
fn default_shell_reads_env() {
    let shell = std::env::var("SHELL").unwrap();
    // SHELL が設定されている前提のテスト
}

#[test]
fn default_shell_unset() {
    // temp::env で SHELL を一時的に除去してテスト
}
```

## SDIT での統計

- テスト数: 575
- smell-allow サプレス: 27件
- 検出: 0件（severity 1）
