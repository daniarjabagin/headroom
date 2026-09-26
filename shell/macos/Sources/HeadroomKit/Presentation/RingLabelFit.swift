public struct RingLabelFit: Sendable, Hashable {
    public static let amountSize = 13.0
    public static let captionSize = 9.0
    public static let minimumScale = 0.5
    static let glyphHeight = 0.8
    static let clearance = 0.04
    static let digitAdvance = 0.62
    static let narrowAdvance = 0.3
    static let upperAdvance = 0.72
    static let lowerAdvance = 0.6

    public let width: Double
    public let fontSize: Double

    public static func make(text: String, ringSize: Double, amountSize: Double, captionSize: Double?) -> RingLabelFit {
        let block = (amountSize + (captionSize ?? 0)) * glyphHeight
        let width = chord(radius: ringSize / 2 * DonutGeometry.holeRatio, height: block) * (1 - clearance)
        let natural = estimatedWidth(text, fontSize: amountSize)
        let scale = natural > width ? max(minimumScale, width / natural) : 1
        return RingLabelFit(width: width, fontSize: amountSize * scale)
    }

    static func chord(radius: Double, height: Double) -> Double {
        let half = height / 2
        return 2 * max(0, radius * radius - half * half).squareRoot()
    }

    static func estimatedWidth(_ text: String, fontSize: Double) -> Double {
        text.reduce(0) { $0 + advance($1) } * fontSize
    }

    static func advance(_ character: Character) -> Double {
        if character.isNumber { return digitAdvance }
        if character.isWhitespace || character == "." || character == "," { return narrowAdvance }
        if character.isUppercase { return upperAdvance }
        return lowerAdvance
    }
}
