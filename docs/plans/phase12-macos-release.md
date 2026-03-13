# Phase 12: macOS リリースビルド調査・提案

**概要**: macOS向けの本番ビルドに必要なバンドルアセットとビルド手順を**調査し、提案する**フェーズ。
実際のCI/CD構築や配布自動化は本フェーズのスコープ外とし、別フェーズで行う。

**状態**: **完了**

**完了条件**:
1. .app バンドルに必要なアセット一覧とその作成方法を文書化する
2. コード署名・公証に必要な手順と前提条件を文書化する
3. ビルド手順（手動）の提案書を `docs/plans/` に作成する

## Phase 12.1: バンドルアセット調査

| タスク | 詳細 |
|---|---|
| .app バンドル構造の調査 | `Contents/MacOS/`, `Contents/Resources/`, `Contents/Info.plist` の必須構成を文書化 |
| Info.plist 必須キーの調査 | CFBundleIdentifier, NSHighResolutionCapable, NSSupportsAutomaticGraphicsSwitching, LSMinimumSystemVersion 等 |
| アイコン要件の調査 | macOS アプリアイコンの必須サイズ一覧、.icns 生成手順（iconutil）、デザインガイドライン |
| entitlements 要件の調査 | wgpu/Metal 使用時に必要な entitlement、Hardened Runtime との互換性 |
| Universal Binary 要件 | x86_64 + aarch64 のクロスコンパイル方法、`lipo` による結合手順 |

## Phase 12.2: コード署名・公証手順の調査

| タスク | 詳細 |
|---|---|
| Developer ID 証明書の確認 | Apple Developer Program で必要な証明書の種類と取得手順 |
| コード署名手順の文書化 | `codesign --options runtime` の詳細オプション、署名対象ファイルの特定 |
| 公証手順の文書化 | `xcrun notarytool submit` のワークフロー、App-specific password の設定方法 |
| DMG 作成手順の文書化 | `create-dmg` の使い方、DMG 自体の署名・公証手順 |

## Phase 12.3: ビルド手順提案書の作成

| タスク | 詳細 |
|---|---|
| 手動ビルド手順書 | 開発者がローカルで .app → 署名 → 公証 → DMG を実行できる手順書 |
| CI/CD 自動化の設計提案 | GitHub Actions での自動化に必要な Secrets、ワークフロー構成の提案（実装はしない） |
| 配布チャネルの提案 | GitHub Releases, Homebrew Cask Tap, 直接ダウンロード等の選択肢と推奨 |

## セキュリティレビュー結果

| 重要度 | ID | 内容 | 対応 |
|---|---|---|---|
| High | H-1 | Release entitlements に不要な権限（カメラ・マイク等）が含まれていた | **修正済み**: apple-events のみに絞った |
| High | H-2 | Debug entitlements にも同じ不要権限 | **修正済み**: apple-events + disable-library-validation のみに |
| Medium | M-1 | Makefile の app ターゲットが entitlements を使用していない | ad-hoc 署名では不要。配布署名は CI/CD で行う想定（ガイド文書に記載済み） |
| Low | L-1 | `NSRequiresAquaSystemAppearance` が `<string>NO</string>` (正しくは `<false/>`) | **修正済み** |
| Low | L-2 | 不要な `NSMainNibFile` 空文字列キー | **修正済み**: 削除 |
| Low | L-3 | 非推奨の `NSLocationAlwaysUsageDescription` | **修正済み**: 削除 |
| Info | I-1 | `byte_end` クランプ後の UTF-8 境界保証（cosmic-text が保証するため実質リスクなし） | 対応不要 |
| Info | I-2 | `f32::MAX` をバッファ幅に渡す意図（折り返しなしの慣用パターン、コメント記載済み） | 対応不要 |
| Info | I-3 | Sandbox 無効を明示していない | 対応不要（Mac App Store 外配布） |
| Info | I-4 | ドキュメントの公証コマンド例に平文パスワード | CI 実装時に環境変数化 |
| Info | I-5 | Makefile 変数のクォート | 現状値にスペースなし、変更時に対応 |

## 依存関係

Phase 11（メニュー等のmacOSネイティブ機能が揃った状態でリリース準備）

## リファレンス

- `refs/alacritty/Makefile` — バンドル作成の参考実装
- `refs/alacritty/extra/osx/Alacritty.app/Contents/Info.plist` — Info.plist の参考
- `refs/ghostty/` — Ghostty の macOS バンドル構成

## 成果物

- `docs/plans/phase12-macos-release-guide.md` — 調査結果と提案をまとめた文書
- `extra/macos/SDIT.app/Contents/Info.plist` — .app バンドルテンプレート
- `extra/macos/SDIT.entitlements` — Release 用 entitlements
- `extra/macos/SDITDebug.entitlements` — Debug 用 entitlements
- `extra/macos/SDIT.app/Contents/Resources/sdit.icns` — プレースホルダアイコン
- `Makefile` — app / dmg ビルドターゲット
