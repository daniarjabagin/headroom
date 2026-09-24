public enum BrandMark {
    public static let icon = SVGIcon(
        origin: PlanePoint(x: 0, y: 0), width: 16, height: 16, commands: plates.flatMap(outline))

    static let plates: [[PlanePoint]] = [
        [point(2.48, 4.88), point(5.2, 4.08), point(5.2, 13.52), point(2.48, 14.32)],
        [point(6.64, 2.32), point(9.36, 1.52), point(9.36, 13.68), point(6.64, 14.48)],
        [point(10.8, 10.16), point(13.52, 9.36), point(13.52, 12.08), point(10.8, 12.88)],
        [point(10.8, 4.72), point(13.52, 3.92), point(13.52, 7.28), point(10.8, 8.08)],
    ]

    private static func point(_ x: Double, _ y: Double) -> PlanePoint {
        PlanePoint(x: x, y: y)
    }

    private static func outline(_ corners: [PlanePoint]) -> [SVGPathCommand] {
        guard let first = corners.first else { return [] }
        return [.move(first)] + corners.dropFirst().map { .line($0) } + [.close]
    }
}
