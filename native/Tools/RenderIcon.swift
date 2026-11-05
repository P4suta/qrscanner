import AppKit
import Foundation

@main
struct RenderIcon {
    @MainActor
    static func main() throws {
        guard CommandLine.arguments.count == 2 else { throw IconError.missingDirectory }
        let directory = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        for size in [16, 32, 128, 256, 512] {
            for scale in [1, 2] {
                let pixels = size * scale
                let suffix = scale == 2 ? "@2x" : ""
                let image = try render(pixels: pixels)
                try image.write(to: directory.appendingPathComponent("icon_\(size)x\(size)\(suffix).png"), options: .atomic)
            }
        }
    }

    @MainActor
    private static func render(pixels: Int) throws -> Data {
        guard let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
                                           bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                                           colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
              let graphics = NSGraphicsContext(bitmapImageRep: bitmap) else { throw IconError.renderingFailed }
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = graphics
        defer { NSGraphicsContext.restoreGraphicsState() }
        graphics.cgContext.scaleBy(x: CGFloat(pixels) / 1024, y: CGFloat(pixels) / 1024)
        let tile = NSBezierPath(roundedRect: NSRect(x: 76, y: 76, width: 872, height: 872), xRadius: 192, yRadius: 192)
        guard let gradient = NSGradient(starting: NSColor(srgbRed: 0.21, green: 0.53, blue: 1, alpha: 1),
                                        ending: NSColor(srgbRed: 0.10, green: 0.28, blue: 0.87, alpha: 1)) else {
            throw IconError.renderingFailed
        }
        gradient.draw(in: tile, angle: -60)
        for point in [NSPoint(x: 232, y: 570), NSPoint(x: 570, y: 570), NSPoint(x: 232, y: 232)] {
            finder(at: point)
        }
        NSColor.white.setStroke()
        let arrow = NSBezierPath()
        arrow.lineWidth = 40
        arrow.lineCapStyle = .round
        arrow.lineJoinStyle = .round
        arrow.move(to: NSPoint(x: 601, y: 267))
        arrow.line(to: NSPoint(x: 774, y: 440))
        arrow.move(to: NSPoint(x: 638, y: 440))
        arrow.line(to: NSPoint(x: 774, y: 440))
        arrow.line(to: NSPoint(x: 774, y: 304))
        arrow.stroke()
        guard let data = bitmap.representation(using: .png, properties: [:]) else { throw IconError.renderingFailed }
        return data
    }

    @MainActor
    private static func finder(at origin: NSPoint) {
        let box = NSRect(origin: origin, size: NSSize(width: 222, height: 222))
        NSColor.white.setFill()
        NSBezierPath(roundedRect: box, xRadius: 28, yRadius: 28).fill()
        NSColor(srgbRed: 0.14, green: 0.36, blue: 0.91, alpha: 1).setFill()
        NSBezierPath(roundedRect: box.insetBy(dx: 34, dy: 34), xRadius: 8, yRadius: 8).fill()
        NSColor.white.setFill()
        NSBezierPath(roundedRect: box.insetBy(dx: 72, dy: 72), xRadius: 5, yRadius: 5).fill()
    }
}

enum IconError: Error {
    case missingDirectory
    case renderingFailed
}
