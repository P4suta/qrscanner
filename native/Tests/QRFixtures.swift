import CoreGraphics
import XCTest

enum QRFixtures {
    static let greekGreeting = "\u{039A}\u{03B1}\u{03BB}\u{03B7}\u{03BC}\u{03AD}\u{03C1}\u{03B1}"

    static let alphabet = """
    111111101111001111111
    100000100111001000001
    101110100101101011101
    101110101110101011101
    101110100111001011101
    100000100100001000001
    111111101010101111111
    000000000010100000000
    001011101100110001001
    101110010001010000000
    001100101000101010110
    110101011101010000010
    001101111000101011110
    000000001001110101000
    111111100010101100001
    100000101111010111101
    101110101011010100001
    101110100110111101010
    101110101000101011101
    100000100110110100011
    111111100000000010101
    """

    static let hello = """
    111111100000001111111
    100000100010101000001
    101110100101001011101
    101110100110101011101
    101110101010101011101
    100000100000101000001
    111111101010101111111
    000000001110000000000
    001100111100011010000
    001110000011000101110
    010101110101000001111
    110010011001111010110
    000010111100000100100
    000000001111001110001
    111111101110101100100
    100000100010011111101
    101110100100001100000
    101110101110100011000
    101110101100010010000
    100000100001101010110
    111111100101110110000
    """

    static func canvas(width: Int, height: Int) throws -> CGContext {
        let context = try XCTUnwrap(CGContext(data: nil, width: width, height: height, bitsPerComponent: 8,
                                             bytesPerRow: width * 4, space: CGColorSpaceCreateDeviceRGB(),
                                             bitmapInfo: CGBitmapInfo.byteOrder32Little.rawValue | CGImageAlphaInfo.premultipliedFirst.rawValue))
        context.setFillColor(gray: 1, alpha: 1)
        context.fill(CGRect(x: 0, y: 0, width: width, height: height))
        context.interpolationQuality = .none
        return context
    }

    static func draw(_ matrix: String, on context: CGContext, scale: CGFloat, origin: CGPoint, rotated: Bool = false) {
        context.saveGState()
        defer { context.restoreGState() }
        context.translateBy(x: origin.x, y: origin.y)
        if rotated { context.rotate(by: .pi / 2) }
        context.setFillColor(gray: 0, alpha: 1)
        let rows = matrix.split(separator: "\n")
        for (row, line) in rows.enumerated() {
            for (column, module) in line.enumerated() where module == "1" {
                context.fill(CGRect(x: CGFloat(column + 4) * scale, y: CGFloat(rows.count + 3 - row) * scale,
                                    width: scale, height: scale))
            }
        }
    }
}
