public struct ShareCardStyle: Sendable, Hashable {
    public static let logicalWidth = 600.0
    public static let logicalHeight = 315.0
    public static let scale = 2.0
    public static let dimAlpha = 0.58
    public static let lineAlpha = 0.12
    public static let accent: UInt32 = 0xA9B5FF

    public static let paper = ShareCardStyle(
        background: 0xF0F0ED, foreground: 0x151617, card: 0xF8F8F8, track: 0xDEDEDE, good: 0x007AFF,
        warning: 0xFFCC00, critical: 0xFF3B30, neutral: 0x8E8E93)
    public static let graphite = ShareCardStyle(
        background: 0x151617, foreground: 0xF0F0ED, card: 0x242424, track: 0x3A3A3C, good: 0x0A84FF,
        warning: 0xFFD60A, critical: 0xFF453A, neutral: 0x98989D)

    public let background: UInt32
    public let foreground: UInt32
    public let card: UInt32
    public let track: UInt32
    public let good: UInt32
    public let warning: UInt32
    public let critical: UInt32
    public let neutral: UInt32

    public static func make(dark: Bool) -> ShareCardStyle {
        dark ? graphite : paper
    }

    public func tone(_ tone: Tone) -> UInt32 {
        switch tone {
        case .good: good
        case .warning: warning
        case .critical: critical
        case .neutral: neutral
        }
    }
}
