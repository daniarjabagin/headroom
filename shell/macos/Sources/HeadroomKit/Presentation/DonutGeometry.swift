import Foundation

public struct DonutSegment: Sendable, Hashable {
    public let index: Int
    public let start: Double
    public let end: Double
    public let gap: Bool
}

public struct DonutGeometry: Sendable, Hashable {
    public static let holeRatio = 0.618
    public static let startAngle = -Double.pi / 2
    static let gapRatio = 2.0 / 104
    static let cornerRatio = 0.15
    static let minWidthRatio = 3.0 / 104
    static let fullTurn = 2 * Double.pi
    static let quarterTurn = Double.pi / 2
    static let angleEpsilon = 1e-9

    public let center: Double
    public let outer: Double
    public let inner: Double
    public let gap: Double
    public let corner: Double

    public init(size: Double) {
        outer = size / 2
        center = outer
        let thickness = outer * (1 - Self.holeRatio)
        inner = outer - thickness
        gap = Self.gapRatio * size
        corner = Self.cornerRatio * thickness
    }

    static var minimumFraction: Double {
        let unit = DonutGeometry(size: 1)
        return (minWidthRatio + unit.gap) / unit.inner / fullTurn
    }

    public static func visibleFractions(_ values: [Int64]) -> [Double] {
        let doubles = values.map { Double(max(0, $0)) }
        let total = doubles.reduce(0, +)
        guard total > 0 else { return values.map { _ in 0 } }
        let minimum = values.count > 1 ? min(1 / Double(values.count), minimumFraction) : 0
        return spread(doubles, minimum: minimum)
    }

    private static func spread(_ values: [Double], minimum: Double) -> [Double] {
        var raised = Set<Int>()
        var fractions = shares(values, raised: raised, minimum: minimum)
        while let index = fractions.indices.first(where: { !raised.contains($0) && fractions[$0] < minimum }) {
            raised.insert(index)
            fractions = shares(values, raised: raised, minimum: minimum)
        }
        return fractions
    }

    private static func shares(_ values: [Double], raised: Set<Int>, minimum: Double) -> [Double] {
        let freeTotal = values.indices.filter { !raised.contains($0) }.map { values[$0] }.reduce(0, +)
        let share = 1 - Double(raised.count) * minimum
        return values.indices.map { index in
            if raised.contains(index) { return minimum }
            return freeTotal > 0 ? values[index] / freeTotal * share : 0
        }
    }

    public static func segments(_ fractions: [Double], reveal: Double = 1) -> [DonutSegment] {
        guard reveal > 0 else { return [] }
        if fractions.count == 1 {
            let end = startAngle + min(reveal, fractions[0]) * fullTurn
            return end > startAngle ? [DonutSegment(index: 0, start: startAngle, end: end, gap: false)] : []
        }
        let limit = startAngle + reveal * fullTurn
        var angle = startAngle
        var result: [DonutSegment] = []
        for (index, fraction) in fractions.enumerated() {
            let start = angle
            angle += fraction * fullTurn
            let end = min(angle, limit)
            if end > start { result.append(DonutSegment(index: index, start: start, end: end, gap: true)) }
        }
        return result
    }
}
