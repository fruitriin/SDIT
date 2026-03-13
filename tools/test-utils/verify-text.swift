#!/usr/bin/env swift
/// verify-text.swift
///
/// SDIT スクリーンショットのテキストレンダリング品質を自動検証する。
/// OCR 照合・輝度分析・SSIM 比較を一括実行し、コンパクトなテキストレポートを返す。
/// エージェントが画像を読む必要をなくし、トークン消費を最小化する。
///
/// Usage:
///   verify-text <image> <expected-text> [options]
///
/// Options:
///   --region x,y,w,h      検査領域（画像ピクセル座標）
///   --reference <path>     対照群画像（SSIM 比較用、render-text で生成）
///   --cells <json-file>    セル境界 JSON（render-text --cell-info の出力を保存したもの）
///   --edge-margin <px>     右端検査マージン（default: 6、@2x 想定）
///   --ssim-threshold <f>   SSIM 閾値（default: 0.3）
///   --luminance-threshold <f>  右端輝度閾値（default: 0.02）
///
/// Output: 構造化テキストレポート（stdout）
/// Exit codes:
///   0 — 全チェック PASS
///   1 — エラー（引数不正等）
///   3 — いずれかのチェック FAIL

import CoreGraphics
import Foundation
import ImageIO
import Vision

// MARK: - 引数パース

struct VerifyOptions {
    var imagePath = ""
    var expectedText = ""
    var region: (Int, Int, Int, Int)? = nil
    var referencePath: String? = nil
    var cellsJsonPath: String? = nil
    var edgeMargin = 6         // pixels (@2x)
    var ssimThreshold: Float = 0.3
    var luminanceThreshold: Float = 0.02
}

func parseArgs() -> VerifyOptions {
    var opts = VerifyOptions()
    let args = Array(CommandLine.arguments.dropFirst())
    var positional: [String] = []

    var i = 0
    while i < args.count {
        switch args[i] {
        case "--region":
            i += 1
            guard i < args.count else { fputs("Error: --region requires x,y,w,h\n", stderr); exit(1) }
            let parts = args[i].split(separator: ",").compactMap { Int($0) }
            guard parts.count == 4 else { fputs("Error: --region format: x,y,w,h\n", stderr); exit(1) }
            opts.region = (parts[0], parts[1], parts[2], parts[3])
        case "--reference":
            i += 1
            guard i < args.count else { fputs("Error: --reference requires path\n", stderr); exit(1) }
            opts.referencePath = args[i]
        case "--cells":
            i += 1
            guard i < args.count else { fputs("Error: --cells requires path\n", stderr); exit(1) }
            opts.cellsJsonPath = args[i]
        case "--edge-margin":
            i += 1
            guard i < args.count, let v = Int(args[i]) else { fputs("Error: --edge-margin requires int\n", stderr); exit(1) }
            opts.edgeMargin = v
        case "--ssim-threshold":
            i += 1
            guard i < args.count, let v = Float(args[i]) else { fputs("Error: --ssim-threshold requires float\n", stderr); exit(1) }
            opts.ssimThreshold = v
        case "--luminance-threshold":
            i += 1
            guard i < args.count, let v = Float(args[i]) else { fputs("Error: --luminance-threshold requires float\n", stderr); exit(1) }
            opts.luminanceThreshold = v
        default:
            positional.append(args[i])
        }
        i += 1
    }

    guard positional.count == 2 else {
        fputs("Usage: verify-text <image> <expected-text> [options]\n", stderr)
        fputs("Options:\n", stderr)
        fputs("  --region x,y,w,h       Crop region\n", stderr)
        fputs("  --reference <path>      Reference image for SSIM\n", stderr)
        fputs("  --cells <json-file>     Cell boundaries JSON\n", stderr)
        fputs("  --edge-margin <px>      Right edge margin (default: 6)\n", stderr)
        fputs("  --ssim-threshold <f>    SSIM threshold (default: 0.3)\n", stderr)
        fputs("  --luminance-threshold <f>  Luminance threshold (default: 0.02)\n", stderr)
        exit(1)
    }

    opts.imagePath = positional[0]
    opts.expectedText = positional[1]
    return opts
}

// MARK: - 画像読み込み

func loadImage(path: String) -> CGImage? {
    guard let provider = CGDataProvider(filename: path),
          let image = CGImage(pngDataProviderSource: provider,
                              decode: nil, shouldInterpolate: false,
                              intent: .defaultIntent) else {
        return nil
    }
    return image
}

func cropImage(_ image: CGImage, region: (Int, Int, Int, Int)) -> CGImage? {
    let rect = CGRect(x: region.0, y: region.1, width: region.2, height: region.3)
    return image.cropping(to: rect)
}

// MARK: - ピクセルデータ取得

struct PixelData {
    let width: Int
    let height: Int
    let data: [UInt8]  // RGBA

    func luminance(x: Int, y: Int) -> Float {
        let offset = (y * width + x) * 4
        guard offset + 2 < data.count else { return 0 }
        // sRGB luminance
        return (0.2126 * Float(data[offset]) +
                0.7152 * Float(data[offset + 1]) +
                0.0722 * Float(data[offset + 2])) / 255.0
    }

    func avgLuminance(rect: (Int, Int, Int, Int)) -> Float {
        let (rx, ry, rw, rh) = rect
        var sum: Float = 0
        var count = 0
        for py in ry..<min(ry + rh, height) {
            for px in rx..<min(rx + rw, width) {
                sum += luminance(x: px, y: py)
                count += 1
            }
        }
        return count > 0 ? sum / Float(count) : 0
    }
}

func getPixelData(_ image: CGImage) -> PixelData? {
    let w = image.width
    let h = image.height
    var pixelData = [UInt8](repeating: 0, count: w * h * 4)
    let colorSpace = CGColorSpaceCreateDeviceRGB()
    guard let ctx = CGContext(
        data: &pixelData,
        width: w, height: h,
        bitsPerComponent: 8,
        bytesPerRow: w * 4,
        space: colorSpace,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else { return nil }
    ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))
    return PixelData(width: w, height: h, data: pixelData)
}

// MARK: - Check 1: OCR 照合

struct OcrResult {
    let recognized: String
    let pass: Bool
    let confidence: Float
}

func checkOcr(image: CGImage, expected: String) -> OcrResult {
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.recognitionLanguages = ["ja", "en"]
    request.usesLanguageCorrection = false  // 言語補正なしで raw 認識

    let handler = VNImageRequestHandler(cgImage: image, options: [:])
    do {
        try handler.perform([request])
    } catch {
        return OcrResult(recognized: "(error)", pass: false, confidence: 0)
    }

    guard let results = request.results, !results.isEmpty else {
        return OcrResult(recognized: "(none)", pass: false, confidence: 0)
    }

    // 全行を結合
    var recognized = ""
    var totalConfidence: Float = 0
    for obs in results {
        if let candidate = obs.topCandidates(1).first {
            if !recognized.isEmpty { recognized += " " }
            recognized += candidate.string
            totalConfidence += candidate.confidence
        }
    }
    let avgConf = totalConfidence / Float(results.count)

    // 正規化比較: スペース・句読点の揺れを吸収
    let normalize = { (s: String) -> String in
        s.replacingOccurrences(of: "\\s+", with: "", options: .regularExpression)
         .replacingOccurrences(of: "　", with: "")
    }
    let pass = normalize(recognized).contains(normalize(expected)) ||
               normalize(expected).contains(normalize(recognized))

    return OcrResult(recognized: recognized, pass: pass, confidence: avgConf)
}

// MARK: - Check 2: 右端輝度分析

struct CellInfo {
    let char: String
    let cells: Int
    let x: Float
    let w: Float
}

struct LuminanceResult {
    let pass: Bool
    let failures: [(String, String, Float)]  // (char, reason, value)
}

func parseCellsJson(path: String) -> [CellInfo]? {
    guard let data = FileManager.default.contents(atPath: path),
          let json = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else {
        return nil
    }
    return json.compactMap { dict in
        guard let char = dict["char"] as? String,
              let cells = dict["cells"] as? Int,
              let x = (dict["x"] as? NSNumber)?.floatValue,
              let w = (dict["w"] as? NSNumber)?.floatValue else { return nil }
        return CellInfo(char: char, cells: cells, x: x, w: w)
    }
}

func checkLuminance(pixels: PixelData, cells: [CellInfo], edgeMargin: Int, threshold: Float, scale: Float) -> LuminanceResult {
    var failures: [(String, String, Float)] = []

    // 背景輝度の推定: 画像の四隅から取得
    let cornerSize = max(4, edgeMargin)
    let bgSamples = [
        pixels.avgLuminance(rect: (0, 0, cornerSize, cornerSize)),
        pixels.avgLuminance(rect: (pixels.width - cornerSize, 0, cornerSize, cornerSize)),
        pixels.avgLuminance(rect: (0, pixels.height - cornerSize, cornerSize, cornerSize)),
        pixels.avgLuminance(rect: (pixels.width - cornerSize, pixels.height - cornerSize, cornerSize, cornerSize))
    ]
    let bgLum = bgSamples.reduce(0, +) / Float(bgSamples.count)

    for cell in cells {
        // スペースはスキップ
        if cell.char.trimmingCharacters(in: .whitespaces).isEmpty { continue }
        if cell.char == "　" { continue }  // 全角スペース

        let cellLeft = Int(cell.x * scale)
        let cellWidth = Int(cell.w * scale)
        let cellTop = 0
        let cellHeight = pixels.height

        guard cellWidth > 0 && cellHeight > 0 else { continue }

        // Check A: セル内にインク（非背景ピクセル）があるか
        // セルの中央 60% の領域で検査（端は自然に空くため）
        let innerMarginX = cellWidth / 5
        let innerX = cellLeft + innerMarginX
        let innerW = cellWidth - innerMarginX * 2
        let innerMarginY = cellHeight / 5
        let innerY = cellTop + innerMarginY
        let innerH = cellHeight - innerMarginY * 2

        guard innerW > 0 && innerH > 0 else { continue }

        let centerLum = pixels.avgLuminance(rect: (innerX, innerY, innerW, innerH))
        let inkPresence = abs(centerLum - bgLum)

        if inkPresence < threshold {
            failures.append((cell.char, "no_ink", inkPresence))
            continue
        }

        // Check B: 右端クリッピング検出
        // セルの右半分と左半分のインク密度を比較
        // 大幅な左右非対称はクリッピングを示唆
        let halfW = cellWidth / 2
        let leftHalfLum = pixels.avgLuminance(rect: (cellLeft, cellTop, halfW, cellHeight))
        let rightHalfLum = pixels.avgLuminance(rect: (cellLeft + halfW, cellTop, halfW, cellHeight))

        let leftInk = abs(leftHalfLum - bgLum)
        let rightInk = abs(rightHalfLum - bgLum)

        // 左にインクがあるのに右にほぼないなら → クリッピングの可能性
        if leftInk > threshold * 3 && rightInk < threshold {
            failures.append((cell.char, "right_clip", rightInk))
        }
    }

    return LuminanceResult(pass: failures.isEmpty, failures: failures)
}

// MARK: - Check 3: SSIM 比較

struct SsimResult {
    let pass: Bool
    let score: Float        // 全体 or 最小セルスコア
    let cellScores: [(String, Float)]  // セル単位スコア（cells 指定時のみ）
}

func computeSSIMRegion(pixels1: PixelData, pixels2: PixelData, rect: (Int, Int, Int, Int)) -> Float {
    let (rx, ry, rw, rh) = rect
    let w = min(rw, min(pixels1.width - rx, pixels2.width - rx))
    let h = min(rh, min(pixels1.height - ry, pixels2.height - ry))
    guard w > 0 && h > 0 else { return 0 }

    let c1: Float = 6.5025    // (0.01 * 255)^2
    let c2: Float = 58.5225   // (0.03 * 255)^2
    let n = Float(w * h)

    var sumL1: Float = 0, sumL2: Float = 0
    for y in ry..<(ry + h) {
        for x in rx..<(rx + w) {
            sumL1 += pixels1.luminance(x: x, y: y)
            sumL2 += pixels2.luminance(x: x, y: y)
        }
    }
    let mu1 = sumL1 / n
    let mu2 = sumL2 / n

    var s1sq: Float = 0, s2sq: Float = 0, s12: Float = 0
    for y in ry..<(ry + h) {
        for x in rx..<(rx + w) {
            let d1 = pixels1.luminance(x: x, y: y) - mu1
            let d2 = pixels2.luminance(x: x, y: y) - mu2
            s1sq += d1 * d1
            s2sq += d2 * d2
            s12 += d1 * d2
        }
    }
    s1sq /= n; s2sq /= n; s12 /= n

    let num = (2 * mu1 * mu2 + c1) * (2 * s12 + c2)
    let den = (mu1 * mu1 + mu2 * mu2 + c1) * (s1sq + s2sq + c2)
    return den > 0 ? num / den : 0
}

func checkSSIM(image: CGImage, referenceImage: CGImage, threshold: Float, cells: [CellInfo]?, scale: Float) -> SsimResult {
    guard let p1 = getPixelData(image),
          let p2 = getPixelData(referenceImage) else {
        return SsimResult(pass: false, score: 0, cellScores: [])
    }

    // セル情報がある場合: セル単位 SSIM（背景の影響を排除）
    if let cells = cells, !cells.isEmpty {
        var cellScores: [(String, Float)] = []
        var minScore: Float = 1.0

        for cell in cells {
            if cell.char.trimmingCharacters(in: .whitespaces).isEmpty { continue }
            if cell.char == "　" { continue }

            let cx = Int(cell.x * scale)
            let cw = Int(cell.w * scale)
            let rect = (cx, 0, cw, min(p1.height, p2.height))
            let score = computeSSIMRegion(pixels1: p1, pixels2: p2, rect: rect)
            cellScores.append((cell.char, score))
            if score < minScore { minScore = score }
        }

        let pass = minScore >= threshold
        return SsimResult(pass: pass, score: minScore, cellScores: cellScores)
    }

    // セル情報なし: 全体 SSIM（サイズ不一致の場合はスキップ）
    guard p1.width == p2.width && p1.height == p2.height else {
        fputs("Warning: image size mismatch (\(p1.width)x\(p1.height) vs \(p2.width)x\(p2.height)), SSIM requires same size\n", stderr)
        return SsimResult(pass: false, score: 0, cellScores: [])
    }

    let score = computeSSIMRegion(pixels1: p1, pixels2: p2, rect: (0, 0, p1.width, p1.height))
    return SsimResult(pass: score >= threshold, score: score, cellScores: [])
}

// MARK: - Main

let opts = parseArgs()

// 画像読み込み
guard let fullImage = loadImage(path: opts.imagePath) else {
    fputs("Error: cannot load '\(opts.imagePath)'\n", stderr)
    exit(1)
}

let image: CGImage
if let r = opts.region {
    guard let cropped = cropImage(fullImage, region: r) else {
        fputs("Error: crop failed\n", stderr)
        exit(1)
    }
    image = cropped
} else {
    image = fullImage
}

var allPass = true
var report = ""

// --- Check 1: OCR ---
let ocr = checkOcr(image: image, expected: opts.expectedText)
let ocrStatus = ocr.pass ? "PASS" : "FAIL"
if !ocr.pass { allPass = false }
report += "[OCR] \(ocrStatus) | expected: \"\(opts.expectedText)\" | recognized: \"\(ocr.recognized)\" | confidence: \(String(format: "%.2f", ocr.confidence))\n"

// --- Check 2: Luminance (if cells provided) ---
if let cellsPath = opts.cellsJsonPath {
    guard let cells = parseCellsJson(path: cellsPath) else {
        fputs("Error: cannot parse cells JSON '\(cellsPath)'\n", stderr)
        exit(1)
    }
    guard let pixels = getPixelData(image) else {
        fputs("Error: cannot get pixel data\n", stderr)
        exit(1)
    }

    // scale を推定: render-text のセル座標は論理座標、画像は @2x
    let scale: Float = 2.0

    let lum = checkLuminance(pixels: pixels, cells: cells, edgeMargin: opts.edgeMargin, threshold: opts.luminanceThreshold, scale: scale)
    let lumStatus = lum.pass ? "PASS" : "FAIL"
    if !lum.pass { allPass = false }
    report += "[LUMINANCE] \(lumStatus) | cells: \(cells.count)"
    if !lum.failures.isEmpty {
        let failChars = lum.failures.map { "\($0.0)(\($0.1)=\(String(format: "%.3f", $0.2)))" }.joined(separator: ", ")
        report += " | failures: \(failChars)"
    }
    report += "\n"
} else {
    report += "[LUMINANCE] SKIP (no --cells provided)\n"
}

// --- Check 3: SSIM (if reference provided) ---
if let refPath = opts.referencePath {
    guard let refImage = loadImage(path: refPath) else {
        fputs("Error: cannot load reference '\(refPath)'\n", stderr)
        exit(1)
    }
    // cells があればセル単位 SSIM で精度向上
    let cellsForSSIM: [CellInfo]?
    if let cellsPath = opts.cellsJsonPath {
        cellsForSSIM = parseCellsJson(path: cellsPath)
    } else {
        cellsForSSIM = nil
    }
    let scale: Float = 2.0
    let ssim = checkSSIM(image: image, referenceImage: refImage, threshold: opts.ssimThreshold, cells: cellsForSSIM, scale: scale)
    let ssimStatus = ssim.pass ? "PASS" : "FAIL"
    if !ssim.pass { allPass = false }
    let mode = (cellsForSSIM != nil && !ssim.cellScores.isEmpty) ? "per-cell min" : "global"
    report += "[SSIM] \(ssimStatus) | \(mode): \(String(format: "%.4f", ssim.score)) | threshold: \(opts.ssimThreshold)"
    if !ssim.cellScores.isEmpty {
        let failCells = ssim.cellScores.filter { $0.1 < opts.ssimThreshold }
        if !failCells.isEmpty {
            let details = failCells.map { "\($0.0)(\(String(format: "%.2f", $0.1)))" }.joined(separator: ", ")
            report += " | low: \(details)"
        }
    }
    report += "\n"
} else {
    report += "[SSIM] SKIP (no --reference provided)\n"
}

// --- Summary ---
report += "[RESULT] \(allPass ? "PASS" : "FAIL")"
print(report)

exit(allPass ? 0 : 3)
