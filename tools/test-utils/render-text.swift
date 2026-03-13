#!/usr/bin/env swift
/// render-text.swift
///
/// CoreText を使ってテキストを PNG にレンダリングする対照群生成ツール。
/// SDIT の CJK レンダリング品質を評価する際の「正解画像」を生成する。
///
/// Usage: render-text [options] <text> <output-path>
///
/// Options:
///   --font <name>       フォント名 (default: Menlo)
///   --size <pt>         フォントサイズ (default: 14)
///   --bg <hex>          背景色 (default: 282828)
///   --fg <hex>          テキスト色 (default: EBDBB2)
///   --padding <px>      パディング (default: 8)
///   --scale <factor>    Retina スケール (default: 2)
///   --mono              等幅グリッド描画（ターミナル風）
///   --cell-info         セル境界情報を JSON で出力
///
/// Exit codes:
///   0 — 成功
///   1 — 引数不正 / フォント未発見 / 書き出し失敗

import CoreGraphics
import CoreText
import Foundation
import ImageIO

// MARK: - 引数パース

struct Options {
    var fontName = "Menlo"
    var fontSize: CGFloat = 14
    var bgColor: (CGFloat, CGFloat, CGFloat) = (0x28/255.0, 0x28/255.0, 0x28/255.0)
    var fgColor: (CGFloat, CGFloat, CGFloat) = (0xEB/255.0, 0xDB/255.0, 0xB2/255.0)
    var padding: CGFloat = 8
    var scale: CGFloat = 2
    var monoGrid = false
    var cellInfo = false
    var text = ""
    var outputPath = ""
}

func parseHexColor(_ hex: String) -> (CGFloat, CGFloat, CGFloat)? {
    let h = hex.hasPrefix("#") ? String(hex.dropFirst()) : hex
    guard h.count == 6, let val = UInt32(h, radix: 16) else { return nil }
    return (
        CGFloat((val >> 16) & 0xFF) / 255.0,
        CGFloat((val >> 8) & 0xFF) / 255.0,
        CGFloat(val & 0xFF) / 255.0
    )
}

func parseArgs() -> Options {
    var opts = Options()
    let args = Array(CommandLine.arguments.dropFirst())
    var positional: [String] = []

    var i = 0
    while i < args.count {
        switch args[i] {
        case "--font":
            i += 1; guard i < args.count else { fputs("Error: --font requires value\n", stderr); exit(1) }
            opts.fontName = args[i]
        case "--size":
            i += 1; guard i < args.count, let v = Double(args[i]) else { fputs("Error: --size requires number\n", stderr); exit(1) }
            opts.fontSize = CGFloat(v)
        case "--bg":
            i += 1; guard i < args.count, let c = parseHexColor(args[i]) else { fputs("Error: --bg requires hex color\n", stderr); exit(1) }
            opts.bgColor = c
        case "--fg":
            i += 1; guard i < args.count, let c = parseHexColor(args[i]) else { fputs("Error: --fg requires hex color\n", stderr); exit(1) }
            opts.fgColor = c
        case "--padding":
            i += 1; guard i < args.count, let v = Double(args[i]) else { fputs("Error: --padding requires number\n", stderr); exit(1) }
            opts.padding = CGFloat(v)
        case "--scale":
            i += 1; guard i < args.count, let v = Double(args[i]) else { fputs("Error: --scale requires number\n", stderr); exit(1) }
            opts.scale = CGFloat(v)
        case "--mono":
            opts.monoGrid = true
        case "--cell-info":
            opts.cellInfo = true
        default:
            positional.append(args[i])
        }
        i += 1
    }

    guard positional.count == 2 else {
        fputs("Usage: render-text [options] <text> <output-path>\n", stderr)
        fputs("Options:\n", stderr)
        fputs("  --font <name>    Font name (default: Menlo)\n", stderr)
        fputs("  --size <pt>      Font size (default: 14)\n", stderr)
        fputs("  --bg <hex>       Background color (default: 282828)\n", stderr)
        fputs("  --fg <hex>       Text color (default: EBDBB2)\n", stderr)
        fputs("  --padding <px>   Padding (default: 8)\n", stderr)
        fputs("  --scale <n>      Retina scale (default: 2)\n", stderr)
        fputs("  --mono           Monospace grid rendering\n", stderr)
        fputs("  --cell-info      Output cell boundary info as JSON\n", stderr)
        exit(1)
    }

    opts.text = positional[0]
    opts.outputPath = positional[1]
    return opts
}

// MARK: - 出力パスバリデーション

func validateOutputPath(_ path: String) {
    let resolved = URL(fileURLWithPath: path).standardized.path
    let cwd = FileManager.default.currentDirectoryPath
    guard resolved.hasPrefix(cwd + "/") || resolved == cwd else {
        fputs("Error: output path must be under working directory (\(cwd))\n", stderr)
        exit(1)
    }
}

// MARK: - Unicode 幅判定

/// East Asian Width に基づいて文字が全角（2セル）かどうかを判定
func isFullWidth(_ scalar: Unicode.Scalar) -> Bool {
    let v = scalar.value
    // CJK Unified Ideographs
    if v >= 0x4E00 && v <= 0x9FFF { return true }
    // CJK Extension A
    if v >= 0x3400 && v <= 0x4DBF { return true }
    // CJK Extension B-I
    if v >= 0x20000 && v <= 0x323AF { return true }
    // CJK Compatibility Ideographs
    if v >= 0xF900 && v <= 0xFAFF { return true }
    // Hiragana
    if v >= 0x3040 && v <= 0x309F { return true }
    // Katakana
    if v >= 0x30A0 && v <= 0x30FF { return true }
    // Halfwidth and Fullwidth Forms (fullwidth range)
    if v >= 0xFF01 && v <= 0xFF60 { return true }
    // Hangul Syllables
    if v >= 0xAC00 && v <= 0xD7AF { return true }
    // CJK Symbols and Punctuation
    if v >= 0x3000 && v <= 0x303F { return true }
    // Enclosed CJK Letters
    if v >= 0x3200 && v <= 0x32FF { return true }
    // CJK Compatibility
    if v >= 0x3300 && v <= 0x33FF { return true }
    // Katakana Phonetic Extensions
    if v >= 0x31F0 && v <= 0x31FF { return true }
    return false
}

/// 文字のセル幅を返す（1 or 2）
func cellWidth(_ char: Character) -> Int {
    for scalar in char.unicodeScalars {
        if isFullWidth(scalar) { return 2 }
    }
    return 1
}

// MARK: - レンダリング

struct CellBoundary {
    let character: String
    let cellWidth: Int
    let x: CGFloat
    let width: CGFloat
}

func render(opts: Options) -> (CGImage, [CellBoundary])? {
    // フォント取得
    guard let ctFont = CTFontCreateWithName(opts.fontName as CFString, opts.fontSize * opts.scale, nil) as CTFont? else {
        fputs("Error: font '\(opts.fontName)' not found\n", stderr)
        return nil
    }

    let scaledPadding = opts.padding * opts.scale

    if opts.monoGrid {
        return renderMonoGrid(ctFont: ctFont, opts: opts, scaledPadding: scaledPadding)
    } else {
        return renderNatural(ctFont: ctFont, opts: opts, scaledPadding: scaledPadding)
    }
}

/// 等幅グリッドレンダリング（ターミナル風）
func renderMonoGrid(ctFont: CTFont, opts: Options, scaledPadding: CGFloat) -> (CGImage, [CellBoundary])? {
    // 半角セル幅を計算（"M" の advance width）
    var glyph: CGGlyph = 0
    var advance = CGSize.zero
    let mChar: [UniChar] = [0x004D] // 'M'
    CTFontGetGlyphsForCharacters(ctFont, mChar, &glyph, 1)
    CTFontGetAdvancesForGlyphs(ctFont, .horizontal, &glyph, &advance, 1)
    let halfCellWidth = ceil(advance.width)

    let lineHeight = ceil(CTFontGetAscent(ctFont) + CTFontGetDescent(ctFont) + CTFontGetLeading(ctFont))
    let ascent = CTFontGetAscent(ctFont)

    // 全文字のセル数を計算
    let characters = Array(opts.text)
    var totalCells = 0
    var cells: [(Character, Int, CGFloat)] = [] // (char, cellWidth, x)
    for ch in characters {
        let cw = cellWidth(ch)
        let x = scaledPadding + CGFloat(totalCells) * halfCellWidth
        cells.append((ch, cw, x))
        totalCells += cw
    }

    let canvasWidth = Int(scaledPadding * 2 + CGFloat(totalCells) * halfCellWidth)
    let canvasHeight = Int(scaledPadding * 2 + lineHeight)

    // CGContext 作成
    let colorSpace = CGColorSpaceCreateDeviceRGB()
    guard let ctx = CGContext(
        data: nil,
        width: canvasWidth,
        height: canvasHeight,
        bitsPerComponent: 8,
        bytesPerRow: canvasWidth * 4,
        space: colorSpace,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else {
        fputs("Error: failed to create CGContext\n", stderr)
        return nil
    }

    // 背景塗りつぶし
    ctx.setFillColor(red: opts.bgColor.0, green: opts.bgColor.1, blue: opts.bgColor.2, alpha: 1.0)
    ctx.fill(CGRect(x: 0, y: 0, width: canvasWidth, height: canvasHeight))

    // テキスト描画
    ctx.setFillColor(red: opts.fgColor.0, green: opts.fgColor.1, blue: opts.fgColor.2, alpha: 1.0)
    // CoreGraphics の座標系は左下原点
    // baselineY をフリップ座標で計算
    let flippedBaselineY = CGFloat(canvasHeight) - scaledPadding - ascent

    var boundaries: [CellBoundary] = []

    for (ch, cw, x) in cells {
        let str = String(ch)
        let attrStr = CFAttributedStringCreateMutable(nil, 0)!
        CFAttributedStringReplaceString(attrStr, CFRange(location: 0, length: 0), str as CFString)
        CFAttributedStringSetAttribute(attrStr, CFRange(location: 0, length: CFAttributedStringGetLength(attrStr)), kCTFontAttributeName, ctFont)
        CFAttributedStringSetAttribute(attrStr, CFRange(location: 0, length: CFAttributedStringGetLength(attrStr)), kCTForegroundColorFromContextAttributeName, kCFBooleanTrue)

        let line = CTLineCreateWithAttributedString(attrStr)
        let charBounds = CTLineGetBoundsWithOptions(line, [])

        // セル中央に配置
        let cellPixelWidth = CGFloat(cw) * halfCellWidth
        let charOffset = (cellPixelWidth - charBounds.width) / 2 - charBounds.origin.x

        ctx.textPosition = CGPoint(x: x + charOffset, y: flippedBaselineY)
        CTLineDraw(line, ctx)

        boundaries.append(CellBoundary(
            character: str,
            cellWidth: cw,
            x: x / opts.scale,
            width: cellPixelWidth / opts.scale
        ))
    }

    guard let image = ctx.makeImage() else {
        fputs("Error: failed to create image from context\n", stderr)
        return nil
    }

    return (image, boundaries)
}

/// 自然なテキストレンダリング（CoreText のカーニング・リガチャを使用）
func renderNatural(ctFont: CTFont, opts: Options, scaledPadding: CGFloat) -> (CGImage, [CellBoundary])? {
    // 属性付き文字列作成
    let attrStr = CFAttributedStringCreateMutable(nil, 0)!
    CFAttributedStringReplaceString(attrStr, CFRange(location: 0, length: 0), opts.text as CFString)
    let fullRange = CFRange(location: 0, length: CFAttributedStringGetLength(attrStr))
    CFAttributedStringSetAttribute(attrStr, fullRange, kCTFontAttributeName, ctFont)
    CFAttributedStringSetAttribute(attrStr, fullRange, kCTForegroundColorFromContextAttributeName, kCFBooleanTrue)

    let line = CTLineCreateWithAttributedString(attrStr)
    let bounds = CTLineGetBoundsWithOptions(line, [])
    let ascent = CTFontGetAscent(ctFont)
    let lineHeight = ceil(CTFontGetAscent(ctFont) + CTFontGetDescent(ctFont) + CTFontGetLeading(ctFont))

    let canvasWidth = Int(ceil(bounds.width) + scaledPadding * 2)
    let canvasHeight = Int(scaledPadding * 2 + lineHeight)

    let colorSpace = CGColorSpaceCreateDeviceRGB()
    guard let ctx = CGContext(
        data: nil,
        width: canvasWidth,
        height: canvasHeight,
        bitsPerComponent: 8,
        bytesPerRow: canvasWidth * 4,
        space: colorSpace,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    ) else {
        fputs("Error: failed to create CGContext\n", stderr)
        return nil
    }

    ctx.setFillColor(red: opts.bgColor.0, green: opts.bgColor.1, blue: opts.bgColor.2, alpha: 1.0)
    ctx.fill(CGRect(x: 0, y: 0, width: canvasWidth, height: canvasHeight))

    ctx.setFillColor(red: opts.fgColor.0, green: opts.fgColor.1, blue: opts.fgColor.2, alpha: 1.0)
    let flippedBaselineY = CGFloat(canvasHeight) - scaledPadding - ascent

    ctx.textPosition = CGPoint(x: scaledPadding - bounds.origin.x, y: flippedBaselineY)
    CTLineDraw(line, ctx)

    guard let image = ctx.makeImage() else {
        fputs("Error: failed to create image from context\n", stderr)
        return nil
    }

    return (image, [])
}

// MARK: - PNG 書き出し

func writePNG(image: CGImage, path: String) -> Bool {
    let url = URL(fileURLWithPath: path)
    let dir = url.deletingLastPathComponent()
    try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)

    guard let dest = CGImageDestinationCreateWithURL(url as CFURL, "public.png" as CFString, 1, nil) else {
        fputs("Error: failed to create image destination at '\(path)'\n", stderr)
        return false
    }

    CGImageDestinationAddImage(dest, image, nil)

    guard CGImageDestinationFinalize(dest) else {
        fputs("Error: failed to write PNG to '\(path)'\n", stderr)
        return false
    }

    return true
}

// MARK: - Main

let opts = parseArgs()
validateOutputPath(opts.outputPath)

guard let (image, boundaries) = render(opts: opts) else {
    exit(1)
}

guard writePNG(image: image, path: opts.outputPath) else {
    exit(1)
}

print("Rendered: \(opts.outputPath) (\(image.width)x\(image.height)px)")

if opts.cellInfo && !boundaries.isEmpty {
    // JSON でセル境界情報を出力
    var json = "[\n"
    for (i, b) in boundaries.enumerated() {
        let escaped = b.character.replacingOccurrences(of: "\\", with: "\\\\").replacingOccurrences(of: "\"", with: "\\\"")
        json += "  {\"char\": \"\(escaped)\", \"cells\": \(b.cellWidth), \"x\": \(b.x), \"w\": \(b.width)}"
        json += i < boundaries.count - 1 ? ",\n" : "\n"
    }
    json += "]"
    print(json)
}
