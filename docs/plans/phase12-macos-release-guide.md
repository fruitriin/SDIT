# Phase 12: macOS リリースビルドガイド

> Alacritty / WezTerm / Ghostty の実装を調査し、SDIT 向けに提案をまとめた文書。

---

## 1. .app バンドル構造

### 必須ディレクトリ構成

```
SDIT.app/
└── Contents/
    ├── Info.plist          # アプリケーションメタデータ（必須）
    ├── MacOS/
    │   └── sdit            # 実行バイナリ
    └── Resources/
        └── sdit.icns       # アプリケーションアイコン
```

### SDIT 固有の追加アセット（任意）

```
SDIT.app/
└── Contents/
    ├── Resources/
    │   ├── sdit.icns
    │   └── sdit.info       # terminfo エントリ（将来）
    └── _CodeSignature/     # codesign が自動生成
```

Alacritty は manpage や shell completions を Resources に同梱。
SDIT では初期リリースではバイナリ + アイコンのみで十分。

---

## 2. Info.plist 必須キー

### 最小限の必須キー

| キー | 値 | 説明 |
|---|---|---|
| `CFBundleDevelopmentRegion` | `en` | 開発言語 |
| `CFBundleExecutable` | `sdit` | バイナリ名 |
| `CFBundleIdentifier` | `com.sdit.terminal` | 一意識別子（逆ドメイン形式） |
| `CFBundleInfoDictionaryVersion` | `6.0` | plist フォーマットバージョン |
| `CFBundleName` | `SDIT` | アプリ名 |
| `CFBundlePackageType` | `APPL` | アプリケーションパッケージ |
| `CFBundleShortVersionString` | `0.1.0` | ユーザー向けバージョン |
| `CFBundleVersion` | `1` | ビルド番号 |
| `CFBundleIconFile` | `sdit.icns` | アイコンファイル名 |

### 推奨キー

| キー | 値 | 説明 |
|---|---|---|
| `NSHighResolutionCapable` | `true` | Retina 対応（wgpu で必須） |
| `NSSupportsAutomaticGraphicsSwitching` | `true` | 内蔵/外部 GPU 自動切り替え |
| `NSRequiresAquaSystemAppearance` | `NO` | ダークモード対応 |
| `CFBundleDisplayName` | `SDIT` | Finder 表示名 |
| `CFBundleSupportedPlatforms` | `["MacOSX"]` | サポートプラットフォーム |
| `LSMinimumSystemVersion` | `13.0` | 最小 OS バージョン（macOS Ventura） |
| `LSApplicationCategoryType` | `public.app-category.utilities` | App Store カテゴリ |

### プライバシー Usage Description キー（ターミナルとして推奨）

ターミナル内で動くプログラムが各種権限を要求する可能性があるため、
Alacritty / WezTerm / Ghostty のいずれもプライバシー Description を網羅的に記述している。

| キー | 用途 |
|---|---|
| `NSAppleEventsUsageDescription` | AppleScript アクセス |
| `NSCameraUsageDescription` | カメラアクセス |
| `NSMicrophoneUsageDescription` | マイクアクセス |
| `NSContactsUsageDescription` | 連絡先アクセス |
| `NSCalendarsUsageDescription` | カレンダーアクセス |
| `NSRemindersUsageDescription` | リマインダーアクセス |
| `NSLocationUsageDescription` | 位置情報アクセス |
| `NSLocationWhenInUseUsageDescription` | 位置情報（使用中） |
| `NSLocationAlwaysUsageDescription` | 位置情報（常時） |
| `NSSystemAdministrationUsageDescription` | 管理者権限 |
| `NSBluetoothAlwaysUsageDescription` | Bluetooth |
| `NSDocumentsFolderUsageDescription` | Documents フォルダ |
| `NSDownloadsFolderUsageDescription` | Downloads フォルダ |
| `NSLocalNetworkUsageDescription` | ローカルネットワーク |

テンプレート: `An application in SDIT would like to access {resource}.`

---

## 3. Entitlements（権限宣言）

### Release 用 entitlements（最小限）

SDIT 自身はカメラ・マイク・位置情報等に直接アクセスしないため、
entitlements は必要最小限に絞る（セキュリティレビュー H-1 対応）。

Alacritty/WezTerm/Ghostty はターミナル内プロセスのための権限を網羅的に宣言しているが、
entitlements は Hardened Runtime の権限宣言であり、PTY 子プロセスの権限とは独立。
ターミナルプロセス自体に不要な権限を宣言する必要はない。

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- AppleScript 連携（ターミナル内プロセスからのオートメーション要求） -->
    <key>com.apple.security.automation.apple-events</key>
    <true/>
</dict>
</plist>
```

### Debug 用 entitlements（追加）

```xml
    <!-- Debug: 署名されていない dylib のロードを許可 -->
    <key>com.apple.security.cs.disable-library-validation</key>
    <true/>
```

> Ghostty の例: Debug ビルドでは `disable-library-validation` を追加し、
> 署名されていないダイナミックライブラリのロードを許可している。

### SDIT 固有の考慮

- wgpu (Metal) は追加の entitlement 不要（Metal API はサンドボックスなしで利用可能）
- Hardened Runtime (`--options runtime`) は公証に必須だが、Metal との互換性問題なし
- PTY 操作に特別な entitlement は不要
- 将来権限が必要になった場合は、その時点で最小限を追加する

---

## 4. アイコン要件

### 必須サイズ一覧（.icns）

| サイズ | 用途 |
|---|---|
| 16x16, 16x16@2x | Finder リスト表示 |
| 32x32, 32x32@2x | Finder アイコン表示 |
| 128x128, 128x128@2x | Finder 大アイコン |
| 256x256, 256x256@2x | Finder 特大 |
| 512x512, 512x512@2x | App Store / Launchpad |

### 生成手順

```bash
# 1. 1024x1024 の PNG マスターアイコンを用意
# 2. iconset ディレクトリを作成
mkdir sdit.iconset
sips -z 16 16     icon_1024.png --out sdit.iconset/icon_16x16.png
sips -z 32 32     icon_1024.png --out sdit.iconset/icon_16x16@2x.png
sips -z 32 32     icon_1024.png --out sdit.iconset/icon_32x32.png
sips -z 64 64     icon_1024.png --out sdit.iconset/icon_32x32@2x.png
sips -z 128 128   icon_1024.png --out sdit.iconset/icon_128x128.png
sips -z 256 256   icon_1024.png --out sdit.iconset/icon_128x128@2x.png
sips -z 256 256   icon_1024.png --out sdit.iconset/icon_256x256.png
sips -z 512 512   icon_1024.png --out sdit.iconset/icon_256x256@2x.png
sips -z 512 512   icon_1024.png --out sdit.iconset/icon_512x512.png
cp icon_1024.png  sdit.iconset/icon_512x512@2x.png

# 3. .icns に変換
iconutil -c icns sdit.iconset -o sdit.icns
```

> 初期リリースではプレースホルダアイコンでも可。

---

## 5. Universal Binary（x86_64 + aarch64）

### ビルド手順（Alacritty 方式）

```bash
# 最小デプロイターゲット（macOS 13 Ventura = Metal 3 対応）
export MACOSX_DEPLOYMENT_TARGET="13.0"

# 各アーキテクチャでビルド
cargo build --release --target=x86_64-apple-darwin
cargo build --release --target=aarch64-apple-darwin

# lipo で結合
lipo target/{x86_64,aarch64}-apple-darwin/release/sdit \
     -create -output target/release/sdit-universal
```

### 前提条件

- `rustup target add x86_64-apple-darwin aarch64-apple-darwin`
- Xcode CLT がインストール済み
- Apple Silicon Mac では x86_64 は Rosetta 2 経由でテスト可能

---

## 6. コード署名

### Ad-hoc 署名（ローカル開発用）

```bash
# Alacritty と同じ方式
codesign --remove-signature "SDIT.app"
codesign --force --deep --sign - "SDIT.app"
```

### Developer ID 署名（配布用）

```bash
# 前提: Apple Developer Program に登録済み
# 証明書: "Developer ID Application: Your Name (TEAMID)"

codesign --force --deep --sign "Developer ID Application: YOUR_NAME (TEAMID)" \
    --options runtime \
    --entitlements SDIT.entitlements \
    "SDIT.app"
```

- `--options runtime`: Hardened Runtime を有効化（公証に必須）
- `--entitlements`: 権限宣言ファイルを指定
- `--deep`: 内部のフレームワーク/dylib も署名（SDIT は Pure Rust なので通常不要だが安全策）

---

## 7. 公証（Notarization）

### 手順

```bash
# 1. .app を ZIP に圧縮
ditto -c -k --keepParent "SDIT.app" "SDIT.zip"

# 2. 公証に提出
xcrun notarytool submit "SDIT.zip" \
    --apple-id "your@email.com" \
    --team-id "TEAMID" \
    --password "app-specific-password" \
    --wait

# 3. 公証チケットをステープル
xcrun stapler staple "SDIT.app"
```

### 前提条件

- Apple Developer Program 登録（年額 $99 USD）
- App-specific password の生成（appleid.apple.com → セキュリティ → App 用パスワード）
- Developer ID Application 証明書がキーチェーンに存在

### 公証失敗時のデバッグ

```bash
# 公証ログの確認
xcrun notarytool log <submission-id> \
    --apple-id "your@email.com" \
    --team-id "TEAMID" \
    --password "app-specific-password"
```

---

## 8. DMG 作成

### 基本的な DMG（Alacritty 方式）

```bash
# 1. 出力ディレクトリ準備
mkdir -p target/release/osx
cp -R "SDIT.app" target/release/osx/
ln -sf /Applications target/release/osx/Applications

# 2. DMG 作成
hdiutil create target/release/SDIT.dmg \
    -volname "SDIT" \
    -fs HFS+ \
    -srcfolder target/release/osx \
    -ov -format UDZO

# 3. DMG の署名・公証（配布用）
codesign --sign "Developer ID Application: YOUR_NAME (TEAMID)" target/release/SDIT.dmg
xcrun notarytool submit target/release/SDIT.dmg --apple-id ... --team-id ... --password ... --wait
xcrun stapler staple target/release/SDIT.dmg
```

### create-dmg を使った美しい DMG（オプション）

```bash
# brew install create-dmg
create-dmg \
    --volname "SDIT" \
    --volicon "extra/macos/sdit.icns" \
    --window-pos 200 120 \
    --window-size 600 400 \
    --icon-size 100 \
    --icon "SDIT.app" 175 190 \
    --app-drop-link 425 190 \
    --hide-extension "SDIT.app" \
    "SDIT.dmg" \
    "target/release/osx/"
```

---

## 9. 提案: SDIT の Makefile

```makefile
TARGET = sdit
RELEASE_DIR = target/release

APP_NAME = SDIT.app
APP_TEMPLATE = extra/macos/$(APP_NAME)
APP_DIR = $(RELEASE_DIR)/osx
APP_BINARY = $(RELEASE_DIR)/$(TARGET)
APP_BINARY_DIR = $(APP_DIR)/$(APP_NAME)/Contents/MacOS
APP_RESOURCES_DIR = $(APP_DIR)/$(APP_NAME)/Contents/Resources

DMG_NAME = SDIT.dmg

# --- ビルド ---

binary:
	cargo build --release

binary-universal:
	MACOSX_DEPLOYMENT_TARGET="13.0" cargo build --release --target=x86_64-apple-darwin
	MACOSX_DEPLOYMENT_TARGET="13.0" cargo build --release --target=aarch64-apple-darwin
	@lipo target/{x86_64,aarch64}-apple-darwin/release/$(TARGET) -create -output $(APP_BINARY)

# --- .app バンドル ---

app: binary
	@mkdir -p $(APP_BINARY_DIR)
	@mkdir -p $(APP_RESOURCES_DIR)
	@cp -fRp $(APP_TEMPLATE) $(APP_DIR)
	@cp -fp $(APP_BINARY) $(APP_BINARY_DIR)
	@codesign --remove-signature "$(APP_DIR)/$(APP_NAME)"
	@codesign --force --deep --sign - "$(APP_DIR)/$(APP_NAME)"
	@echo "Created '$(APP_NAME)' in '$(APP_DIR)'"

app-universal: binary-universal
	@mkdir -p $(APP_BINARY_DIR)
	@mkdir -p $(APP_RESOURCES_DIR)
	@cp -fRp $(APP_TEMPLATE) $(APP_DIR)
	@cp -fp $(APP_BINARY) $(APP_BINARY_DIR)
	@codesign --remove-signature "$(APP_DIR)/$(APP_NAME)"
	@codesign --force --deep --sign - "$(APP_DIR)/$(APP_NAME)"
	@echo "Created universal '$(APP_NAME)' in '$(APP_DIR)'"

# --- DMG ---

dmg: app
	@ln -sf /Applications $(APP_DIR)/Applications
	@hdiutil create $(APP_DIR)/$(DMG_NAME) \
		-volname "SDIT" \
		-fs HFS+ \
		-srcfolder $(APP_DIR) \
		-ov -format UDZO
	@echo "Created '$(DMG_NAME)' in '$(APP_DIR)'"

# --- クリーン ---

clean:
	cargo clean

.PHONY: binary binary-universal app app-universal dmg clean
```

---

## 10. 提案: CI/CD 自動化（GitHub Actions）

### 必要な Secrets

| Secret 名 | 内容 |
|---|---|
| `APPLE_CERTIFICATE` | Developer ID Application 証明書（.p12, base64） |
| `APPLE_CERTIFICATE_PASSWORD` | 証明書のパスワード |
| `APPLE_ID` | Apple ID メールアドレス |
| `APPLE_TEAM_ID` | Apple Developer Team ID |
| `APPLE_APP_PASSWORD` | App-specific password |
| `KEYCHAIN_PASSWORD` | 一時キーチェーンのパスワード |

### ワークフロー概要

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags: ['v*']

jobs:
  build-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust targets
        run: rustup target add x86_64-apple-darwin aarch64-apple-darwin
      - name: Build universal binary
        run: make binary-universal
      - name: Create .app bundle
        run: make app-universal
      - name: Sign with Developer ID
        run: |
          # キーチェーンセットアップ + codesign
      - name: Notarize
        run: |
          # xcrun notarytool submit + staple
      - name: Create DMG
        run: make dmg
      - name: Upload to GitHub Releases
        uses: softprops/action-gh-release@v1
        with:
          files: target/release/osx/SDIT.dmg
```

> 実装は Phase 12 のスコープ外。別フェーズで行う。

---

## 11. 配布チャネル提案

| チャネル | 推奨度 | メリット | デメリット |
|---|---|---|---|
| **GitHub Releases** | ★★★ | 即座に開始可能、OSS 標準 | 自動更新なし |
| **Homebrew Cask Tap** | ★★★ | `brew install --cask sdit` で一発 | Tap リポジトリの管理が必要 |
| 公式サイト直接DL | ★★ | ブランディング自由 | インフラ構築が必要 |
| Mac App Store | ★ | 発見性が高い | サンドボックス制約が厳しい（PTY 操作に制限） |

### 推奨: GitHub Releases + Homebrew Cask Tap

1. **初期**: GitHub Releases に DMG をアップロード
2. **安定後**: Homebrew Cask Tap を作成（`homebrew-sdit` リポジトリ）

---

## 12. 実装優先度

| 優先度 | アクション | 状態 |
|---|---|---|
| P0 | .app バンドルテンプレート作成（Info.plist + アイコンプレースホルダ） | 本フェーズ |
| P0 | Makefile 作成（app + dmg ターゲット） | 本フェーズ |
| P0 | entitlements ファイル作成（Release + Debug） | 本フェーズ |
| P1 | GitHub Actions ワークフロー実装 | 次フェーズ |
| P1 | 公証パイプライン構築 | 次フェーズ（Apple Developer Program 登録後） |
| P2 | Homebrew Cask Tap 作成 | 安定リリース後 |
| P2 | アイコンデザイン | デザイナーまたはオーナー判断 |

---

## リファレンス

| プロジェクト | 参照ファイル | 主な知見 |
|---|---|---|
| Alacritty | `Makefile`, `extra/osx/` | 簡潔な Makefile ベースのバンドル作成・DMG 生成 |
| WezTerm | `assets/macos/`, `Info.plist` | CFBundleDocumentTypes（シェルスクリプト関連付け） |
| Ghostty | `macos/*.entitlements` | Release/Debug/ReleaseLocal の entitlements 分離パターン |
