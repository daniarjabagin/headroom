#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    extension Tone {
        public var color: Color {
            switch self {
            case .good: Palette.ok
            case .warning: Palette.warn
            case .critical: Palette.crit
            case .neutral: Palette.neutral
            }
        }
    }

    extension SeriesColor {
        var color: Color { Palette.dynamic(light: light, dark: dark) }
    }

    enum Palette {
        static let ok = Color.accentColor
        static let warn = Color(nsColor: .systemYellow)
        static let crit = Color(nsColor: .systemRed)
        static let notice = Color(nsColor: .systemOrange)
        static let neutral = Color(nsColor: .systemGray)
        static let tray = Color(nsColor: .textBackgroundColor)
        static let track = Color.primary.opacity(0.1)
        static let tick = Color.primary.opacity(0.55)
        static let skeleton = Color.primary.opacity(0.08)
        static let separator = Color.primary.opacity(0.1)
        static let segmentTrack = Color.primary.opacity(0.05)
        static let hover = dynamic(light: 0x000000, dark: 0xFFFFFF, lightAlpha: 0.045, darkAlpha: 0.07)
        static let pressed = dynamic(light: 0x000000, dark: 0xFFFFFF, lightAlpha: 0.08, darkAlpha: 0.11)
        static let noticeBackground = dynamic(light: 0xFFF4DF, dark: 0x3B2E1E)
        static let noticeForeground = dynamic(light: 0xA85D00, dark: 0xFFB95C)
        static let noticeBorder = dynamic(light: 0xFF9500, dark: 0xFF9F0A, lightAlpha: 0.18, darkAlpha: 0.18)
        static let noticeTile = dynamic(light: 0xFF9500, dark: 0xFF9F0A, lightAlpha: 0.12, darkAlpha: 0.14)
        static let errorBackground = dynamic(light: 0xFFF0EE, dark: 0x3D2525)
        static let errorForeground = dynamic(light: 0xB42318, dark: 0xFF8F86)
        static let errorBorder = dynamic(light: 0xB42318, dark: 0xFF8F86, lightAlpha: 0.14, darkAlpha: 0.16)
        static let errorTile = dynamic(light: 0xB42318, dark: 0xFF8F86, lightAlpha: 0.1, darkAlpha: 0.12)

        static func dynamic(light: UInt32, dark: UInt32, lightAlpha: Double = 1, darkAlpha: Double = 1) -> Color {
            Color(
                nsColor: NSColor(name: nil) { appearance in
                    let isDark = appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
                    return isDark ? rgb(dark, alpha: darkAlpha) : rgb(light, alpha: lightAlpha)
                })
        }

        private static func rgb(_ hex: UInt32, alpha: Double) -> NSColor {
            NSColor(
                srgbRed: CGFloat((hex >> 16) & 0xFF) / 255, green: CGFloat((hex >> 8) & 0xFF) / 255,
                blue: CGFloat(hex & 0xFF) / 255, alpha: CGFloat(alpha))
        }
    }

    enum Typeface {
        static let title = Font.system(size: 14, weight: .semibold)
        static let label = Font.system(size: 13, weight: .semibold)
        static let body = Font.system(size: 12)
        static let bodyMedium = Font.system(size: 12, weight: .medium)
        static let bodyStrong = Font.system(size: 12, weight: .semibold)
        static let caption = Font.system(size: 11)
        static let captionMedium = Font.system(size: 11, weight: .medium)
        static let captionStrong = Font.system(size: 11, weight: .semibold)
        static let caption2 = Font.system(size: 10)
        static let ring = Font.system(size: 15, weight: .semibold, design: .rounded)
    }

    enum PopupMetrics {
        static let width: CGFloat = 320
        static let cornerRadius: CGFloat = 13
        static let cardRadius: CGFloat = 12
        static let noticeRadius: CGFloat = 9
        static let chipRadius: CGFloat = 6
        static let padding: CGFloat = 14
        static let bottomPadding: CGFloat = 12
        static let sectionGap: CGFloat = 14
        static let headerGap: CGFloat = 4
        static let cardGutter: CGFloat = 5
        static let rowInset: CGFloat = 14
        static let barRowPadding: CGFloat = 10
        static let textRowPadding: CGFloat = 6
        static let rowSpacing: CGFloat = 4
        static let headerLeading: CGFloat = 10
        static let headerTrailing: CGFloat = 4
        static let meterHeight: CGFloat = 5
        static let tickWidth: CGFloat = 2
        static let tickHeight: CGFloat = 9
        static let donutSize: CGFloat = 104
        static let legendGap: CGFloat = 18
        static let legendRowGap: CGFloat = 7
        static let refreshSize: CGFloat = 24
        static let gearSize: CGFloat = 28
        static let screenMargin: CGFloat = 16
        static let tipDelay: Duration = .milliseconds(400)
    }
#endif
