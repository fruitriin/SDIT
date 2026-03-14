#!/usr/bin/env swift
/// clip-image.swift
///
/// PNG 画像の指定領域を切り出す。
/// annotate-grid で座標を確認した後、注目領域だけを切り出して LLM に渡す際に使う。
///
/// Usage:
///   clip-image <input-png> <output-png> --rect x y width height
///   clip-image <input-png> <output-png> --grid-cell col row N
///
/// Exit codes: 0=成功, 1=引数不正/読み込み失敗/書き出し失敗

import CoreGraphics
import Foundation
import ImageIO

// MARK: - オプション

struct Options {
    var inputPath  = ""
    var outputPath = ""
    var rect:     (Int, Int, Int, Int)? = nil  // x, y, width, height
    var gridCell: (Int, Int, Int)? = nil        // col, row, N
}

func parseArgs() -> Options {
    var opts = Options()
    let args = Array(CommandLine.arguments.dropFirst())
    var positional: [String] = []

    var i = 0
    while i < args.count {
        switch args[i] {
        case "--rect":
            guard i + 4 < args.count,
                  let x = Int(args[i + 1]), let y = Int(args[i + 2]),
                  let w = Int(args[i + 3]), let h = Int(args[i + 4]) else {
                fputs("Error: --rect requires x y width height\n", stderr); exit(1)
            }
            opts.rect = (x, y, w, h)
            i += 4
        case "--grid-cell":
            guard i + 3 < args.count,
                  let col = Int(args[i + 1]), let row = Int(args[i + 2]),
                  let n   = Int(args[i + 3]) else {
                fputs("Error: --grid-cell requires col row N\n", stderr); exit(1)
            }
            opts.gridCell = (col, row, n)
            i += 3
        default:
            positional.append(args[i])
        }
        i += 1
    }

    guard positional.count == 2 else {
        fputs("Usage: clip-image <input-png> <output-png> --rect x y width height\n", stderr)
        fputs("       clip-image <input-png> <output-png> --grid-cell col row N\n", stderr)
        exit(1)
    }

    opts.inputPath  = positional[0]
    opts.outputPath = positional[1]

    guard opts.rect != nil || opts.gridCell != nil else {
        fputs("Error: --rect または --grid-cell が必要です\n", stderr)
        exit(1)
    }

    return opts
}

// MARK: - パストラバーサル防止（capture-window.swift L50-58 準拠）

func validatePath(_ path: String, label: String) {
    let resolved = URL(fileURLWithPath: path).standardized.path
    let cwd = FileManager.default.currentDirectoryPath
    guard resolved.hasPrefix(cwd + "/") || resolved == cwd else {
        fputs("Error: \(label) はワーキングディレクトリ配下でなければなりません (\(cwd))\n", stderr)
        fputs("  resolved: \(resolved)\n", stderr)
        exit(1)
    }
}

// MARK: - PNG 書き出し

func writePNG(image: CGImage, path: String) -> Bool {
    let url = URL(fileURLWithPath: path)
    let dir = url.deletingLastPathComponent()
    try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)

    guard let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil) else {
        fputs("Error: 書き出し先の作成に失敗しました '\(path)'\n", stderr)
        return false
    }

    CGImageDestinationAddImage(dest, image, nil)

    guard CGImageDestinationFinalize(dest) else {
        fputs("Error: PNG 書き出しに失敗しました '\(path)'\n", stderr)
        return false
    }

    return true
}

// MARK: - Main

let opts = parseArgs()
validatePath(opts.inputPath,  label: "入力パス")
validatePath(opts.outputPath, label: "出力パス")

// PNG 読み込み（ocr-region.swift 参照実装準拠）
guard let dataProvider = CGDataProvider(filename: opts.inputPath),
      let inputImage = CGImage(pngDataProviderSource: dataProvider,
                               decode: nil, shouldInterpolate: false,
                               intent: .defaultIntent) else {
    fputs("Error: 画像の読み込みに失敗しました '\(opts.inputPath)'\n", stderr)
    exit(1)
}

let imgW = inputImage.width
let imgH = inputImage.height

// クリップ矩形の計算
let clipX: Int
let clipY: Int
let clipW: Int
let clipH: Int

if let (x, y, w, h) = opts.rect {
    clipX = x
    clipY = y
    clipW = w
    clipH = h
} else if let (col, row, n) = opts.gridCell {
    guard n >= 1 else {
        fputs("Error: N は 1 以上が必要です\n", stderr); exit(1)
    }
    guard col < n else {
        fputs("Error: col (\(col)) は N (\(n)) 未満でなければなりません\n", stderr); exit(1)
    }
    guard row < n else {
        fputs("Error: row (\(row)) は N (\(n)) 未満でなければなりません\n", stderr); exit(1)
    }

    let cellW = imgW / n
    let cellH = imgH / n
    clipX = col * cellW
    clipY = row * cellH
    // 最終列・行は余りピクセルを吸収
    clipW = (col == n - 1) ? imgW - clipX : cellW
    clipH = (row == n - 1) ? imgH - clipY : cellH
} else {
    fputs("Error: --rect または --grid-cell が必要です\n", stderr)
    exit(1)
}

// 矩形クロップ（ocr-region.swift L51-58 参照実装準拠）
let cropRect = CGRect(x: clipX, y: clipY, width: clipW, height: clipH)
guard let croppedImage = inputImage.cropping(to: cropRect) else {
    fputs("Error: 画像のクロップに失敗しました \(cropRect)\n", stderr)
    exit(1)
}

guard writePNG(image: croppedImage, path: opts.outputPath) else {
    exit(1)
}

print("Clipped: \(opts.outputPath) (\(croppedImage.width)x\(croppedImage.height)px)")
