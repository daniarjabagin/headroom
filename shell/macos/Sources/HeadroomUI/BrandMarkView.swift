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
        public init() {}

        public var body: some View {
            BrandMarkView(size: MenuBarLabel.height)
                .foregroundStyle(Color.black)
        }
    }
#endif
