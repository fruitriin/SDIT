TARGET = sdit
RELEASE_DIR = target/release

APP_NAME = SDIT.app
APP_TEMPLATE = extra/macos/$(APP_NAME)
APP_DIR = $(RELEASE_DIR)/osx
APP_BINARY = $(RELEASE_DIR)/$(TARGET)
APP_BINARY_DIR = $(APP_DIR)/$(APP_NAME)/Contents/MacOS
APP_RESOURCES_DIR = $(APP_DIR)/$(APP_NAME)/Contents/Resources

DMG_NAME = SDIT.dmg

all: help

help: ## ヘルプを表示する
	@grep -E '^[a-zA-Z._-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

binary: ## リリースバイナリをビルド
	cargo build --release

binary-universal: ## Universal Binary をビルド (x86_64 + aarch64)
	MACOSX_DEPLOYMENT_TARGET="13.0" cargo build --release --target=x86_64-apple-darwin
	MACOSX_DEPLOYMENT_TARGET="13.0" cargo build --release --target=aarch64-apple-darwin
	@lipo target/{x86_64,aarch64}-apple-darwin/release/$(TARGET) -create -output $(APP_BINARY)

app: binary ## SDIT.app を作成 (ネイティブ)
	@mkdir -p $(APP_BINARY_DIR)
	@mkdir -p $(APP_RESOURCES_DIR)
	@cp -fRp $(APP_TEMPLATE)/Contents/Info.plist $(APP_DIR)/$(APP_NAME)/Contents/
	@cp -fRp $(APP_TEMPLATE)/Contents/Resources/* $(APP_RESOURCES_DIR)/
	@cp -fp $(APP_BINARY) $(APP_BINARY_DIR)
	@codesign --remove-signature "$(APP_DIR)/$(APP_NAME)"
	@codesign --force --deep --sign - "$(APP_DIR)/$(APP_NAME)"
	@echo "Created '$(APP_NAME)' in '$(APP_DIR)'"

app-universal: binary-universal ## Universal SDIT.app を作成
	@mkdir -p $(APP_BINARY_DIR)
	@mkdir -p $(APP_RESOURCES_DIR)
	@cp -fRp $(APP_TEMPLATE)/Contents/Info.plist $(APP_DIR)/$(APP_NAME)/Contents/
	@cp -fRp $(APP_TEMPLATE)/Contents/Resources/* $(APP_RESOURCES_DIR)/
	@cp -fp $(APP_BINARY) $(APP_BINARY_DIR)
	@codesign --remove-signature "$(APP_DIR)/$(APP_NAME)"
	@codesign --force --deep --sign - "$(APP_DIR)/$(APP_NAME)"
	@echo "Created universal '$(APP_NAME)' in '$(APP_DIR)'"

dmg: app ## SDIT.dmg を作成
	@mkdir -p $(APP_DIR)
	@ln -sf /Applications $(APP_DIR)/Applications
	@hdiutil create $(APP_DIR)/$(DMG_NAME) \
		-volname "SDIT" \
		-fs HFS+ \
		-srcfolder $(APP_DIR) \
		-ov -format UDZO
	@echo "Created '$(DMG_NAME)' in '$(APP_DIR)'"

dmg-universal: app-universal ## Universal SDIT.dmg を作成
	@mkdir -p $(APP_DIR)
	@ln -sf /Applications $(APP_DIR)/Applications
	@hdiutil create $(APP_DIR)/$(DMG_NAME) \
		-volname "SDIT" \
		-fs HFS+ \
		-srcfolder $(APP_DIR) \
		-ov -format UDZO
	@echo "Created '$(DMG_NAME)' in '$(APP_DIR)'"

clean: ## ビルド成果物を削除
	cargo clean

.PHONY: all help binary binary-universal app app-universal dmg dmg-universal clean
