#if canImport(AppKit)
    import AppKit
    import SwiftUI

    struct TranslucentSurfaceKey: EnvironmentKey {
        static let defaultValue = false
    }

    extension EnvironmentValues {
        var headroomTranslucent: Bool {
            get { self[TranslucentSurfaceKey.self] }
            set { self[TranslucentSurfaceKey.self] = newValue }
        }
    }

    struct PopupSurface: ViewModifier {
        let translucent: Bool

        private var shape: RoundedRectangle {
            RoundedRectangle(cornerRadius: PopupMetrics.cornerRadius, style: .continuous)
        }

        func body(content: Content) -> some View {
            content
                .background { backdrop }
                .overlay(shape.strokeBorder(Palette.popupBorder, lineWidth: 1))
        }

        @ViewBuilder
        private var backdrop: some View {
            if translucent {
                BlurredBackdrop(radius: PopupMetrics.cornerRadius)
            } else {
                shape.fill(Palette.tray)
            }
        }
    }

    struct BlurredBackdrop: NSViewRepresentable {
        let radius: CGFloat

        func makeNSView(context: Context) -> NSVisualEffectView {
            let view = NSVisualEffectView()
            view.material = .menu
            view.blendingMode = .behindWindow
            view.state = .active
            view.maskImage = Self.mask(radius: radius)
            return view
        }

        func updateNSView(_ view: NSVisualEffectView, context: Context) {}

        private static func mask(radius: CGFloat) -> NSImage {
            let edge = radius * 2 + 1
            let image = NSImage(size: NSSize(width: edge, height: edge), flipped: false) { rect in
                NSColor.black.setFill()
                NSBezierPath(roundedRect: rect, xRadius: radius, yRadius: radius).fill()
                return true
            }
            image.capInsets = NSEdgeInsets(top: radius, left: radius, bottom: radius, right: radius)
            image.resizingMode = .stretch
            return image
        }
    }

    struct CardSurface: ViewModifier {
        var radius: CGFloat = PopupMetrics.cardRadius
        @Environment(\.headroomTranslucent) private var translucent

        func body(content: Content) -> some View {
            content.background(
                translucent ? Palette.translucentCard : Palette.card,
                in: RoundedRectangle(cornerRadius: radius, style: .continuous))
        }
    }

    struct FooterSurface: ViewModifier {
        @Environment(\.headroomTranslucent) private var translucent

        func body(content: Content) -> some View {
            content
                .background(translucent ? Color.clear : Palette.footer)
                .overlay(alignment: .top) {
                    Rectangle().fill(Palette.separator).frame(height: 1)
                }
        }
    }

    extension View {
        func popupSurface(translucent: Bool) -> some View {
            modifier(PopupSurface(translucent: translucent))
        }

        func cardSurface(radius: CGFloat = PopupMetrics.cardRadius) -> some View {
            modifier(CardSurface(radius: radius))
        }

        func footerSurface() -> some View {
            modifier(FooterSurface())
        }
    }
#endif
