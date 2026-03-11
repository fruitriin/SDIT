# 進捗表 — Phase 5: 設定・仕上げ

## タスク: Phase 5 — 設定・仕上げ

### Phase 5.1 — TOML設定基盤 + フォント設定
- [x] sdit-config に serde + toml + dirs 依存を追加
- [x] FontConfig struct 定義 + プラットフォーム別デフォルト
- [x] Config struct + Config::load() 実装
- [x] FontContext::new を設定値で初期化するよう変更
- [x] main.rs で Config をロードして使用
- [x] ユニットテスト追加
- [x] cargo fmt + clippy + test 全通過

### Phase 5.2 — カラーテーマ設定
- [x] ColorConfig + 組み込みテーマ定義
- [x] ResolvedColors struct
- [x] main.rs のハードコード色を設定参照に置換
- [x] wcag_contrast_ratio() 実装

### Phase 5.3 — 日本語フォント対応
- [x] fallback_families 設定追加
- [x] WIDE_CHAR/WIDE_CHAR_SPACER 処理（pipeline.rs）
- [x] cell_width_scale による2倍幅描画（シェーダー対応）

### Phase 5.4 — セッション永続化
- [x] AppSnapshot 保存/ロード
- [x] アトミック書き込み（一時ファイル + rename）
- [x] ユニットテスト（roundtrip, 破損時フォールバック）

### Phase 5.5 — 統合テスト強化
- [x] WCAG コントラスト比テスト（全テーマ）
- [x] CJK 幅テスト（wide_char, wrap_at_line_end, unicode_width）

### セキュリティレビュー
- [x] M-1: NaN/Infinity 対策 → clamped_size/clamped_line_height に is_finite チェック
- [x] M-2: TOCTOU 対策 → 一時ファイル名に PID+ナノ秒
- [x] M-3: GPU 防御 → シェーダー内で clamp(1.0, 2.0)
- [x] L-1〜L-5, I-1: Plan に記録済み
