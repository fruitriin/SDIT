# シェルインテグレーション OSC 133 実装知見

## 実装概要

OSC 133 (FinalTerm) シーケンスでシェルのプロンプト境界を検出し、プロンプト間ジャンプを実現する。

## データ構造

```rust
// SemanticZone: OSC 133 のマーカー種別
pub enum SemanticZone {
    PromptStart,          // OSC 133;A
    CommandStart,         // OSC 133;B
    OutputStart,          // OSC 133;C
    CommandEnd(Option<i32>), // OSC 133;D[;exit_code]
}

// SemanticMarker: 行番号 + ゾーン種別
pub struct SemanticMarker {
    pub line: i32,
    pub zone: SemanticZone,
}

// Terminal 側フィールド
pub semantic_markers: VecDeque<SemanticMarker>,
pub shell_integration_enabled: bool,
```

## VecDeque 選択理由と注意点

`semantic_markers` は `VecDeque<SemanticMarker>` として実装されている。
理由: `MAX_SEMANTIC_MARKERS` 超過時に `pop_front()` で古いマーカーを O(1) で削除するため。

**注意**: `Vec::push()` ではなく `VecDeque::push_back()` を使う必要がある。
コンパイラの提案通り `push_back` に修正すること。

## OSC 133 パーサー実装

`osc_dispatch` で `params[0] == b"133"` を検出し、`params[1]` でサブコマンドを判定:

```rust
if params[0] == b"133" {
    let sub = params[1];
    let zone = if sub == b"A" {
        Some(SemanticZone::PromptStart)
    } else if sub == b"B" {
        Some(SemanticZone::CommandStart)
    } else if sub == b"C" {
        Some(SemanticZone::OutputStart)
    } else if sub == b"D" {
        // params[2] に exit_code が続く場合がある
        let exit_code = params.get(2).and_then(|p| std::str::from_utf8(p).ok())
            .and_then(|s| s.parse::<i32>().ok());
        Some(SemanticZone::CommandEnd(exit_code))
    } else {
        None
    };
    if let Some(z) = zone {
        let line = self.grid.cursor.point.line.0;
        self.semantic_markers.push_back(SemanticMarker { line, zone: z });
        if self.semantic_markers.len() > MAX_SEMANTIC_MARKERS {
            self.semantic_markers.pop_front();
        }
    }
}
```

## プロンプトジャンプのスクロール計算

`prev_prompt()` / `next_prompt()` は `SemanticZone::PromptStart` マーカーのみを検索する。

GUI 側 (event_loop.rs) でのスクロール計算:
```
new_offset = (history_size - target_line).clamp(0, history_size)
delta = new_offset - current_offset
grid.scroll_display(Scroll::Delta(delta))
```

display_offset の意味: `0` = 最下部（最新行）、`history_size` = 最上部（最古行）。
target_line が大きい（新しい行）ほど new_offset は小さくなる。

## Shell Integration 有効/無効

- `Terminal::shell_integration_enabled` フィールドで制御
- GUI 側 (app.rs) がセッション生成時と Hot Reload 時に `Config.shell_integration.enabled` を反映する
- 無効時は `osc_dispatch` でのマーカー記録をスキップ

## fish シェルとの互換性

fish は組み込みで OSC 133 A/B/C を出力するため、追加設定不要。
bash/zsh は `precmd`/`PS1` フックによる手動設定が必要。

## テスト一覧

| テスト名 | 内容 |
|---|---|
| `osc_133_prompt_start` | OSC 133;A → PromptStart マーカー記録 |
| `osc_133_command_start` | OSC 133;B → CommandStart マーカー記録 |
| `osc_133_output_start` | OSC 133;C → OutputStart マーカー記録 |
| `osc_133_command_end_with_exit_code` | OSC 133;D;0 → CommandEnd(Some(0)) |
| `osc_133_command_end_no_exit_code` | OSC 133;D → CommandEnd(None) |
| `osc_133_disabled_when_shell_integration_off` | 無効時はマーカーを記録しない |
| `prompt_navigation` | prev_prompt/next_prompt のナビゲーション |
| `semantic_markers_capped` | MAX 超過時に古いマーカーが破棄される |
