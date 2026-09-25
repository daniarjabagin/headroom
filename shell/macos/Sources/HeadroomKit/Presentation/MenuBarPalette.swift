public struct PanelColor: Sendable, Hashable {
    public let red: Double
    public let green: Double
    public let blue: Double
    public let alpha: Double

    public init(hex: UInt32, alpha: Double = 1) {
        red = Double((hex >> 16) & 0xFF) / 255
        green = Double((hex >> 8) & 0xFF) / 255
        blue = Double(hex & 0xFF) / 255
        self.alpha = alpha
    }

    public func withAlpha(_ value: Double) -> PanelColor {
        PanelColor(red: red, green: green, blue: blue, alpha: alpha * value)
    }

    private init(red: Double, green: Double, blue: Double, alpha: Double) {
        self.red = red
        self.green = green
        self.blue = blue
        self.alpha = alpha
    }
}

public enum PanelAppearance: Sendable, Hashable {
    case template, light, dark

    public static func resolve(tone: Tone?, darkPanel: Bool) -> PanelAppearance {
        guard tone == .warning || tone == .critical else { return .template }
        return darkPanel ? .dark : .light
    }
}

public struct MenuBarColors: Sendable, Hashable {
    public let foreground: PanelColor
    public let secondary: PanelColor
    public let track: PanelColor
    public let tick: PanelColor
    public let text: PanelColor
    public let graphic: PanelColor
}

public enum MenuBarPalette {
    public static let trackAlpha = 0.28
    public static let tickAlpha = 0.8
    public static let secondaryAlpha = 0.67
    public static let dimmedOpacity = 140.0 / 255
    public static let pulsePeriod = 2.0

    public static func colors(tone: Tone?, appearance: PanelAppearance) -> MenuBarColors {
        let foreground = foreground(appearance)
        return MenuBarColors(
            foreground: foreground, secondary: foreground.withAlpha(secondaryAlpha),
            track: foreground.withAlpha(trackAlpha), tick: foreground.withAlpha(tickAlpha),
            text: text(tone, appearance: appearance) ?? foreground,
            graphic: graphic(tone, appearance: appearance) ?? foreground)
    }

    static func foreground(_ appearance: PanelAppearance) -> PanelColor {
        switch appearance {
        case .template: PanelColor(hex: 0x000000)
        case .light: PanelColor(hex: 0x000000, alpha: 0.85)
        case .dark: PanelColor(hex: 0xFFFFFF)
        }
    }

    static func text(_ tone: Tone?, appearance: PanelAppearance) -> PanelColor? {
        switch (tone, appearance) {
        case (.warning, .light): PanelColor(hex: 0xB76B00)
        case (.warning, .dark): PanelColor(hex: 0xFFD60A)
        case (.critical, .light): PanelColor(hex: 0xE0281E)
        case (.critical, .dark): PanelColor(hex: 0xFF453A)
        default: nil
        }
    }

    static func graphic(_ tone: Tone?, appearance: PanelAppearance) -> PanelColor? {
        switch (tone, appearance) {
        case (.warning, .light): PanelColor(hex: 0xFFCC00)
        case (.critical, .light): PanelColor(hex: 0xFF3B30)
        default: text(tone, appearance: appearance)
        }
    }
}

public enum MenuBarBar {
    public static let width = 26.0
    public static let height = 5.0
    public static let tickWidth = 2.0
    public static let tickHeight = 9.0

    public static func fillWidth(_ fraction: Double) -> Double {
        guard fraction.isFinite, fraction > 0 else { return 0 }
        return min(width, max(height, fraction * width))
    }

    public static func tickCenter(_ position: Double) -> Double {
        let half = tickWidth / 2
        guard position.isFinite else { return half }
        return min(width - half, max(half, position * width))
    }
}
