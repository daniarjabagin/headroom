#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    @MainActor
    public final class ProviderLogos {
        private static let renderSize: CGFloat = 32

        private let store = ProviderIconStore()
        private var images: [String: Image] = [:]

        public init() {}

        public func image(for provider: String) -> Image? {
            if let cached = images[provider] { return cached }
            guard let icon = store.icon(for: provider), let image = render(icon, provider: provider) else { return nil }
            images[provider] = image
            return image
        }

        private func render(_ icon: SVGIcon, provider: String) -> Image? {
            let tinted = ProviderStyle.brandTintedIcons.contains(provider)
            let color = tinted ? brandColor(provider) : Color.black
            let renderer = ImageRenderer(
                content: SVGIconShape(icon: icon).fill(color).frame(width: Self.renderSize, height: Self.renderSize))
            renderer.scale = 2
            guard let nsImage = renderer.nsImage else { return nil }
            nsImage.isTemplate = !tinted
            let image = Image(nsImage: nsImage)
            return tinted ? image : image.renderingMode(.template)
        }

        private func brandColor(_ provider: String) -> Color {
            let hex = ProviderStyle.seriesColor(for: provider).light
            return Color(
                .sRGB, red: Double((hex >> 16) & 0xFF) / 255, green: Double((hex >> 8) & 0xFF) / 255,
                blue: Double(hex & 0xFF) / 255)
        }
    }
#endif
