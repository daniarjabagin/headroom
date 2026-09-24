import Foundation

struct DonutArc {
    let center: PlanePoint
    let radius: Double
    let from: Double
    let to: Double
    let negative: Bool

    init(center: PlanePoint, radius: Double, from: Double, to: Double, negative: Bool = false) {
        self.center = center
        self.radius = radius
        self.from = from
        self.to = to
        self.negative = negative
    }

    var sweepEnd: Double {
        var end = to
        if negative {
            while end > from { end -= DonutGeometry.fullTurn }
        } else {
            while end < from { end += DonutGeometry.fullTurn }
        }
        return end
    }

    func points(maxStep: Double) -> [PlanePoint] {
        let end = sweepEnd
        let steps = max(1, Int((abs(end - from) / maxStep).rounded(.up)))
        return (0...steps).map { step in
            let angle = from + (end - from) * Double(step) / Double(steps)
            return PlanePoint(x: center.x + radius * cos(angle), y: center.y + radius * sin(angle))
        }
    }
}

extension DonutGeometry {
    static let arcStep = Double.pi / 90

    public func outline(_ segment: DonutSegment) -> [[PlanePoint]]? {
        if !segment.gap && segment.end - segment.start >= Self.fullTurn - Self.angleEpsilon {
            return ringOutline()
        }
        let halfGap = segment.gap ? gap / 2 : 0
        guard let corner = cornerRadius(halfSweep: (segment.end - segment.start) / 2, halfGap: halfGap) else {
            return nil
        }
        let outerOffset = asin((halfGap + corner) / (outer - corner))
        let innerOffset = asin((halfGap + corner) / (inner + corner))
        let arcs = sectorArcs(segment, corner: corner, outerOffset: outerOffset, innerOffset: innerOffset)
        return [arcs.flatMap { $0.points(maxStep: Self.arcStep) }]
    }

    private var centerPoint: PlanePoint { PlanePoint(x: center, y: center) }

    private func ringOutline() -> [[PlanePoint]] {
        let outerLoop = DonutArc(center: centerPoint, radius: outer, from: 0, to: Self.fullTurn)
        let innerLoop = DonutArc(center: centerPoint, radius: inner, from: Self.fullTurn, to: 0, negative: true)
        return [outerLoop.points(maxStep: Self.arcStep), innerLoop.points(maxStep: Self.arcStep)]
    }

    private func cornerRadius(halfSweep: Double, halfGap: Double) -> Double? {
        let sine = sin(min(halfSweep, Self.quarterTurn))
        let innerRoom = (inner * sine - halfGap) / (1 - sine)
        let outerRoom = (outer * sine - halfGap) / (1 + sine)
        let room = min(innerRoom, outerRoom)
        return room < 0 ? nil : min(corner, room)
    }

    private func polar(_ radius: Double, _ angle: Double) -> PlanePoint {
        PlanePoint(x: center + radius * cos(angle), y: center + radius * sin(angle))
    }

    private func sectorArcs(
        _ segment: DonutSegment, corner: Double, outerOffset: Double, innerOffset: Double
    ) -> [DonutArc] {
        let start = segment.start
        let end = segment.end
        let quarter = Self.quarterTurn
        let outerCorner = { (angle: Double) in polar(outer - corner, angle) }
        let innerCorner = { (angle: Double) in polar(inner + corner, angle) }
        return [
            DonutArc(
                center: outerCorner(start + outerOffset), radius: corner, from: start - quarter, to: start + outerOffset
            ),
            DonutArc(
                center: centerPoint, radius: outer, from: start + outerOffset,
                to: max(end - outerOffset, start + outerOffset)),
            DonutArc(
                center: outerCorner(end - outerOffset), radius: corner, from: end - outerOffset, to: end + quarter),
            DonutArc(
                center: innerCorner(end - innerOffset), radius: corner, from: end + quarter,
                to: end - innerOffset + .pi),
            DonutArc(
                center: centerPoint, radius: inner, from: end - innerOffset,
                to: min(start + innerOffset, end - innerOffset), negative: true),
            DonutArc(
                center: innerCorner(start + innerOffset), radius: corner, from: start + innerOffset + .pi,
                to: start + 3 * quarter),
        ]
    }
}
