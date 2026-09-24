public struct MeterSpan: Sendable, Hashable {
    public let offset: Double
    public let width: Double
}

public enum SegmentedMeter {
    public static let gap = 2.0

    public static func spans(count: Int, width: Double, gap: Double = gap) -> [MeterSpan] {
        guard count > 0, width.isFinite, width > 0 else { return [] }
        let each = max(0, (width - gap * Double(count - 1)) / Double(count))
        return (0..<count).map { MeterSpan(offset: Double($0) * (each + gap), width: each) }
    }

    public static func fillWidth(_ fraction: Double, span: Double, minimum: Double) -> Double {
        let clamped = clamp(fraction)
        guard clamped > 0, span > 0 else { return 0 }
        return min(span, max(minimum, span * clamped))
    }

    static func clamp(_ fraction: Double) -> Double {
        fraction.isFinite ? min(1, max(0, fraction)) : 0
    }
}
