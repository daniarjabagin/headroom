public struct SeriesColor: Sendable, Hashable {
    public let light: UInt32
    public let dark: UInt32
}

public enum ProviderStyle {
    public static let brandTintedIcons: Set<String> = ["claude"]

    private static let series: [String: SeriesColor] = [
        "codex": SeriesColor(light: 0x10A37F, dark: 0x10A37F),
        "claude": SeriesColor(light: 0xD97757, dark: 0xD97757),
        "opencode": SeriesColor(light: 0x6E6E73, dark: 0xAEAEB2),
        "openrouter": SeriesColor(light: 0x6467F2, dark: 0x7C7FF5),
        "zai": SeriesColor(light: 0x2D2D2D, dark: 0xD1D1D6),
        "kimi": SeriesColor(light: 0x1D93D2, dark: 0x3AA9E4),
        "minimax": SeriesColor(light: 0xE73562, dark: 0xF0527A),
        "grok": SeriesColor(light: 0x8E8E93, dark: 0x98989D),
        "cline": SeriesColor(light: 0x0F9D9A, dark: 0x2CC3BF),
        "devin": SeriesColor(light: 0x8B5E3C, dark: 0xB08560),
        "copilot": SeriesColor(light: 0xA855F7, dark: 0xB77CF9),
        "cursor": SeriesColor(light: 0x13120A, dark: 0xF5F5F7),
        "antigravity": SeriesColor(light: 0x4285F4, dark: 0x5B96F6),
        "ollama": SeriesColor(light: 0x65A30D, dark: 0x84CC16),
    ]

    private static let fallbackSeries: [SeriesColor] = [
        SeriesColor(light: 0x34C759, dark: 0x30D158),
        SeriesColor(light: 0x5856D6, dark: 0x5E5CE6),
        SeriesColor(light: 0xFF2D55, dark: 0xFF375F),
        SeriesColor(light: 0xA2845E, dark: 0xAC8E68),
    ]

    public static func seriesColor(for provider: String) -> SeriesColor {
        if let known = series[provider] { return known }
        let index = Int(stableHash(provider) % UInt32(fallbackSeries.count))
        return fallbackSeries[index]
    }

    public static func iconResource(for provider: String) -> String? {
        let valid = !provider.isEmpty && provider.unicodeScalars.allSatisfy(isProviderIDScalar)
        return valid ? provider : nil
    }

    static func stableHash(_ value: String) -> UInt32 {
        value.unicodeScalars.reduce(UInt32(0)) { hash, scalar in hash &* 31 &+ scalar.value }
    }

    private static func isProviderIDScalar(_ scalar: Unicode.Scalar) -> Bool {
        switch scalar {
        case "a"..."z", "0"..."9", "_", "-": true
        default: false
        }
    }
}
