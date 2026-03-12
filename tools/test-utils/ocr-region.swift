#!/usr/bin/env swift
/// ocr-region.swift
///
/// PNG 画像の指定領域（または全体）から OCR でテキストを抽出する。
/// macOS Vision framework を使用。
///
/// Usage: ocr-region <image-path>
///        ocr-region <image-path> <x> <y> <width> <height>
///
/// Output: 認識されたテキストを1行ずつ stdout に出力
/// Exit codes:
///   0 — 成功（テキストが1つ以上認識された）
///   1 — エラー（引数不正、画像読み込み失敗等）
///   2 — テキストが認識されなかった

import Foundation
import Vision
import CoreGraphics
import ImageIO

// MARK: - 引数チェック

let args = CommandLine.arguments
guard args.count == 2 || args.count == 6 else {
    fputs("Usage: ocr-region <image-path>\n", stderr)
    fputs("       ocr-region <image-path> <x> <y> <width> <height>\n", stderr)
    exit(1)
}

let imagePath = args[1]

// MARK: - 画像読み込み

guard let dataProvider = CGDataProvider(filename: imagePath),
      let cgImage = CGImage(pngDataProviderSource: dataProvider,
                            decode: nil, shouldInterpolate: false,
                            intent: .defaultIntent) else {
    fputs("Error: failed to load image '\(imagePath)'\n", stderr)
    exit(1)
}

// MARK: - 領域指定（オプション）

let targetImage: CGImage
if args.count == 6 {
    guard let x = Int(args[2]), let y = Int(args[3]),
          let w = Int(args[4]), let h = Int(args[5]) else {
        fputs("Error: invalid region coordinates\n", stderr)
        exit(1)
    }
    let rect = CGRect(x: x, y: y, width: w, height: h)
    guard let cropped = cgImage.cropping(to: rect) else {
        fputs("Error: failed to crop image to \(rect)\n", stderr)
        exit(1)
    }
    targetImage = cropped
} else {
    targetImage = cgImage
}

// MARK: - OCR 実行

let request = VNRecognizeTextRequest()
request.recognitionLevel = .accurate
request.recognitionLanguages = ["ja", "en"]
request.usesLanguageCorrection = true

let handler = VNImageRequestHandler(cgImage: targetImage, options: [:])

do {
    try handler.perform([request])
} catch {
    fputs("Error: OCR failed: \(error.localizedDescription)\n", stderr)
    exit(1)
}

guard let results = request.results, !results.isEmpty else {
    fputs("No text recognized\n", stderr)
    exit(2)
}

// MARK: - 結果出力

for observation in results {
    if let candidate = observation.topCandidates(1).first {
        print(candidate.string)
    }
}
