#if canImport(AppKit)
    import Foundation
    import HeadroomKit
    import Observation
    import SwiftUI

    @MainActor
    @Observable
    final class ProviderIconStore {
        nonisolated private static let folder = "ProviderIcons"

        @ObservationIgnored private var icons: [String: SVGIcon] = [:]
        @ObservationIgnored private var missing: Set<String> = []
        @ObservationIgnored private let directory = ProviderIconStore.locateDirectory()

        func icon(for provider: String) -> SVGIcon? {
            if let icon = icons[provider] { return icon }
            guard !missing.contains(provider), let icon = load(provider) else {
                missing.insert(provider)
                return nil
            }
            icons[provider] = icon
            return icon
        }

        private func load(_ provider: String) -> SVGIcon? {
            guard let name = ProviderStyle.iconResource(for: provider), let directory else { return nil }
            let url = directory.appendingPathComponent("\(name).svg")
            guard let text = try? String(contentsOf: url, encoding: .utf8) else { return nil }
            return try? SVGIcon.parse(text)
        }

        nonisolated private static func locateDirectory() -> URL? {
            let main = Bundle.main
            let roots = [main.resourceURL, main.bundleURL, main.executableURL?.deletingLastPathComponent()]
            return ResourceLocator.directory(
                named: folder, inBundle: ResourceLocator.uiBundleName, roots: roots.compactMap { $0 })
        }
    }

    struct SVGIconShape: Shape {
        let icon: SVGIcon

        func path(in rect: CGRect) -> Path {
            let scale = min(rect.width / icon.width, rect.height / icon.height)
            let offsetX = rect.minX + (rect.width - icon.width * scale) / 2 - icon.origin.x * scale
            let offsetY = rect.minY + (rect.height - icon.height * scale) / 2 - icon.origin.y * scale
            let map = { (point: PlanePoint) in CGPoint(x: offsetX + point.x * scale, y: offsetY + point.y * scale) }
            var path = Path()
            for command in icon.commands {
                switch command {
                case .move(let point): path.move(to: map(point))
                case .line(let point): path.addLine(to: map(point))
                case .cubic(let first, let second, let end):
                    path.addCurve(to: map(end), control1: map(first), control2: map(second))
                case .close: path.closeSubpath()
                }
            }
            return path
        }
    }

    struct ProviderGlyph: View {
        let provider: String
        var size: CGFloat = 16
        @Environment(ProviderIconStore.self) private var store: ProviderIconStore?

        var body: some View {
            Group {
                if let icon = store?.icon(for: provider) {
                    SVGIconShape(icon: icon)
                        .fill(tint, style: FillStyle(eoFill: false, antialiased: true))
                } else {
                    Image(systemName: "cube")
                        .font(.system(size: size * 0.8, weight: .medium))
                        .foregroundStyle(.secondary)
                }
            }
            .frame(width: size, height: size)
            .accessibilityHidden(true)
        }

        private var tint: AnyShapeStyle {
            guard ProviderStyle.brandTintedIcons.contains(provider) else { return AnyShapeStyle(.secondary) }
            return AnyShapeStyle(ProviderStyle.seriesColor(for: provider).color)
        }
    }
#endif
