#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct BrandMarkView: View {
        let size: CGFloat

        var body: some View {
            SVGIconShape(icon: BrandMark.icon)
                .frame(width: size, height: size)
                .accessibilityHidden(true)
        }
    }

    public struct MenuBarMark: View {
        let color: Color

        public init(color: Color) {
            self.color = color
        }

        public var body: some View {
            BrandMarkView(size: MenuBarLabel.height)
                .foregroundStyle(color)
        }
    }
#endif
