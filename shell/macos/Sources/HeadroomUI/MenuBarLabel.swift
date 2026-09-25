#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    extension PanelColor {
        public var color: Color {
            Color(.sRGB, red: red, green: green, blue: blue, opacity: alpha)
        }
    }

    @MainActor
    public final class MenuBarLogos {
        private let store = ProviderIconStore()

        public init() {}

        public func icon(for logo: String) -> SVGIcon? {
            store.icon(for: logo)
        }
    }

    public struct MenuBarSlotView: View {
        let slot: MenuBarSlot
        let colors: MenuBarColors
        let logo: SVGIcon?

        public init(slot: MenuBarSlot, colors: MenuBarColors, logo: SVGIcon?) {
            self.slot = slot
            self.colors = colors
            self.logo = logo
        }

        public var body: some View {
            switch slot {
            case .mark: MenuBarMark(color: colors.text.color)
            case .item(let item): MenuBarLabel(item: item, colors: colors, logo: logo)
            }
        }
    }

    public struct MenuBarLabel: View {
        public static let height: CGFloat = 18

        let item: MenuBarItem
        let colors: MenuBarColors
        let logo: SVGIcon?

        public var body: some View {
            HStack(spacing: 4) {
                if item.logo != nil { source }
                indicator
                if let text = item.text {
                    Text(text)
                        .font(.system(size: 12, weight: .bold))
                        .monospacedDigit()
                        .foregroundStyle(colors.text.color)
                }
            }
            .frame(height: Self.height)
            .fixedSize()
            .opacity(item.stale ? MenuBarPalette.dimmedOpacity : 1)
        }

        private var source: some View {
            HStack(spacing: 2) {
                MenuBarLogo(icon: logo, color: colors.foreground.color)
                if let letter = item.windowLetter {
                    Text(letter)
                        .font(.system(size: 9, weight: .heavy))
                        .foregroundStyle(colors.secondary.color)
                }
            }
        }

        @ViewBuilder private var indicator: some View {
            switch item.indicator {
            case .none: EmptyView()
            case .ring(let fraction):
                MenuBarRing(fraction: fraction, colors: colors).frame(width: 13, height: 13)
            case .bar(let fraction, let tick):
                MenuBarPill(fraction: fraction, tick: tick, colors: colors)
            }
        }
    }

    struct MenuBarLogo: View {
        let icon: SVGIcon?
        let color: Color

        var body: some View {
            Group {
                if let icon {
                    SVGIconShape(icon: icon).fill(color)
                } else {
                    Image(systemName: "cube").font(.system(size: 11, weight: .medium)).foregroundStyle(color)
                }
            }
            .frame(width: 14, height: 14)
        }
    }

    struct MenuBarRing: View {
        let fraction: Double
        let colors: MenuBarColors

        var body: some View {
            ZStack {
                Circle().stroke(colors.track.color, lineWidth: 2.5)
                Circle()
                    .trim(from: 0, to: CGFloat(min(1, max(0, fraction))))
                    .stroke(colors.graphic.color, style: StrokeStyle(lineWidth: 2.5, lineCap: .round))
                    .rotationEffect(.degrees(-90))
            }
            .padding(1.25)
        }
    }

    struct MenuBarPill: View {
        let fraction: Double
        let tick: Double?
        let colors: MenuBarColors

        var body: some View {
            ZStack(alignment: .leading) {
                Capsule().fill(colors.track.color)
                    .frame(width: MenuBarBar.width, height: MenuBarBar.height)
                Capsule().fill(colors.graphic.color)
                    .frame(width: MenuBarBar.fillWidth(fraction), height: MenuBarBar.height)
                if let tick {
                    RoundedRectangle(cornerRadius: 1)
                        .fill(colors.tick.color)
                        .frame(width: MenuBarBar.tickWidth, height: MenuBarBar.tickHeight)
                        .offset(x: MenuBarBar.tickCenter(tick) - MenuBarBar.tickWidth / 2)
                }
            }
            .frame(width: MenuBarBar.width, height: MenuBarBar.tickHeight, alignment: .leading)
        }
    }
#endif
