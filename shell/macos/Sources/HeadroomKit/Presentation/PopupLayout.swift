public struct PopupLayout: Sendable, Hashable {
    public static let normal = PopupLayout(
        density: .normal, contentTop: 12, contentBottom: 10, sectionGap: 10, headerGap: 3, cardGutter: 3,
        barRowPadding: 7, textRowPadding: 5, meterHeight: 5, tickHeight: 9, trendHeight: 18, donutSize: 76,
        glyphSize: 14, spendCardPaddingY: 10, legendGap: 14, segmentPaddingY: 3, caretPaddingY: 5, footerPaddingY: 8,
        gearSize: 24, typeDelta: 0)

    public static let compact = PopupLayout(
        density: .compact, contentTop: 8, contentBottom: 8, sectionGap: 6, headerGap: 2, cardGutter: 2,
        barRowPadding: 4, textRowPadding: 3, meterHeight: 4, tickHeight: 8, trendHeight: 14, donutSize: 64,
        glyphSize: 12, spendCardPaddingY: 6, legendGap: 10, segmentPaddingY: 2, caretPaddingY: 3, footerPaddingY: 5,
        gearSize: 20, typeDelta: -1)

    public let density: Density
    public let contentTop: Double
    public let contentBottom: Double
    public let sectionGap: Double
    public let headerGap: Double
    public let cardGutter: Double
    public let barRowPadding: Double
    public let textRowPadding: Double
    public let meterHeight: Double
    public let tickHeight: Double
    public let trendHeight: Double
    public let donutSize: Double
    public let glyphSize: Double
    public let spendCardPaddingY: Double
    public let legendGap: Double
    public let segmentPaddingY: Double
    public let caretPaddingY: Double
    public let footerPaddingY: Double
    public let gearSize: Double
    public let typeDelta: Double

    public var isCompact: Bool { density == .compact }
    public var trendScale: Double { trendHeight / UsageRows.trendHeight }

    public static func make(_ density: Density) -> PopupLayout {
        density == .compact ? compact : normal
    }

    public func size(_ base: Double) -> Double {
        base + typeDelta
    }
}
