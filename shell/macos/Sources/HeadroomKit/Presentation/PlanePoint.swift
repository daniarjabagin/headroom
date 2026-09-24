public struct PlanePoint: Sendable, Hashable {
    public let x: Double
    public let y: Double

    public init(x: Double, y: Double) {
        self.x = x
        self.y = y
    }
}

extension PlanePoint {
    static func + (lhs: PlanePoint, rhs: PlanePoint) -> PlanePoint {
        PlanePoint(x: lhs.x + rhs.x, y: lhs.y + rhs.y)
    }

    static func - (lhs: PlanePoint, rhs: PlanePoint) -> PlanePoint {
        PlanePoint(x: lhs.x - rhs.x, y: lhs.y - rhs.y)
    }

    static func * (scale: Double, point: PlanePoint) -> PlanePoint {
        PlanePoint(x: scale * point.x, y: scale * point.y)
    }
}
