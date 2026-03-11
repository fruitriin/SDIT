#!/usr/bin/env swift
/// capture-window.swift
///
/// ScreenCaptureKit (macOS 15+) を使って指定プロセスのウィンドウを PNG キャプチャする。
///
/// Usage: capture-window <process-name> <output-path>
/// Exit codes:
///   0 — 成功（PNG を output-path に書き出し）
///   1 — 引数不正 / ウィンドウが見つからない
///   2 — Screen Recording 権限がない

import CoreGraphics
import Foundation
import ScreenCaptureKit

// MARK: - 引数チェック

guard CommandLine.arguments.count == 3 else {
    fputs("Usage: capture-window <process-name> <output-path>\n", stderr)
    exit(1)
}

let targetName = CommandLine.arguments[1]
let outputPath = CommandLine.arguments[2]

// MARK: - プロセス検索

func findPid(named name: String) -> pid_t? {
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/bin/ps")
    task.arguments = ["-eo", "pid,comm"]

    let pipe = Pipe()
    task.standardOutput = pipe
    task.standardError = Pipe()

    do {
        try task.run()
    } catch {
        return nil
    }
    task.waitUntilExit()

    let data = pipe.fileHandleForReading.readDataToEndOfFile()
    guard let output = String(data: data, encoding: .utf8) else { return nil }

    for line in output.split(separator: "\n") {
        let parts = line.trimmingCharacters(in: .whitespaces).split(separator: " ", maxSplits: 1)
        guard parts.count == 2 else { continue }
        let comm = parts[1].trimmingCharacters(in: .whitespaces)
        let basename = URL(fileURLWithPath: String(comm)).lastPathComponent
        if basename == name {
            if let pid = pid_t(parts[0]) {
                return pid
            }
        }
    }
    return nil
}

guard let pid = findPid(named: targetName) else {
    fputs("Error: process '\(targetName)' not found\n", stderr)
    exit(1)
}

// MARK: - ScreenCaptureKit でキャプチャ

// 非同期処理を同期的に待つための DispatchSemaphore
let semaphore = DispatchSemaphore(value: 0)
var captureError: Error?
var captureImage: CGImage?

Task {
    do {
        // 共有コンテンツ一覧を取得
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)

        // 対象プロセスのウィンドウを探す
        let targetWindow = content.windows.first { window in
            guard let app = window.owningApplication else { return false }
            return app.processID == pid
        }

        guard let window = targetWindow else {
            fputs("Error: no on-screen window found for '\(targetName)' (pid=\(pid))\n", stderr)
            semaphore.signal()
            exit(1)
        }

        // フィルター: 対象ウィンドウのみ
        let filter = SCContentFilter(desktopIndependentWindow: window)

        // キャプチャ設定
        let config = SCStreamConfiguration()
        config.width = Int(window.frame.width) * 2  // Retina 対応 @2x
        config.height = Int(window.frame.height) * 2
        config.pixelFormat = kCVPixelFormatType_32BGRA
        config.showsCursor = false

        // スクリーンショット取得（macOS 14.4+ API）
        let image = try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: config)
        captureImage = image
        semaphore.signal()
    } catch let error as SCStreamError where error.code == .userDeclined {
        fputs("Error: Screen Recording permission denied.\n", stderr)
        fputs("System Settings → Privacy & Security → Screen Recording でこのツールを許可してください。\n", stderr)
        fputs("権限付与後に再起動が必要です。\n", stderr)
        captureError = error
        semaphore.signal()
    } catch {
        fputs("Error: capture failed: \(error.localizedDescription)\n", stderr)
        captureError = error
        semaphore.signal()
    }
}

semaphore.wait()

if captureError != nil {
    // SCStreamError.userDeclined は code 1
    let err = captureError! as NSError
    if err.code == 1 {
        exit(2)
    }
    exit(1)
}

guard let image = captureImage else {
    fputs("Error: capture returned no image\n", stderr)
    exit(1)
}

// MARK: - PNG として書き出し

let url = URL(fileURLWithPath: outputPath)

// 出力先ディレクトリを作成
let dir = url.deletingLastPathComponent()
try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)

guard let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil) else {
    fputs("Error: failed to create image destination at '\(outputPath)'\n", stderr)
    exit(1)
}

CGImageDestinationAddImage(dest, image, nil)

guard CGImageDestinationFinalize(dest) else {
    fputs("Error: failed to write PNG to '\(outputPath)'\n", stderr)
    exit(1)
}

print("Captured: \(outputPath) (\(image.width)x\(image.height)px)")
