#if canImport(AppKit)
    import AppKit
    import SwiftUI

    struct GlassSurfaceKey: EnvironmentKey {
        static let defaultValue = false
    }

    extension EnvironmentValues {
        var headroomGlass: Bool {
            get { self[GlassSurfaceKey.self] }
            set { self[GlassSurfaceKey.self] = newValue }
        }
    }

    struct PopupSurface: ViewModifier {
        let translucent: Bool

        static func usesGlass(translucent: Bool) -> Bool {
            #if compiler(>=6.2)
                if #available(macOS 26, *) { return true }
            #endif
            return translucent
        }

        private var shape: RoundedRectangle {
            RoundedRectangle(cornerRadius: PopupMetrics.cornerRadius, style: .continuous)
        }

        @ViewBuilder
        func body(content: Content) -> some View {
            #if compiler(>=6.2)
                if #available(macOS 26, *) {
                    content.glassEffect(translucent ? .clear : .regular, in: shape)
                } else {
                    fallback(content)
                }
            #else
                fallback(content)
            #endif
        }

        @ViewBuilder
        private func fallback(_ content: Content) -> some View {
            if translucent {
                content.background(.regularMaterial, in: shape)
            } else {
                content
                    .background(Palette.tray, in: shape)
                    .overlay(shape.strokeBorder(Color.primary.opacity(0.12), lineWidth: 1))
            }
        }
    }

    struct CardSurface: ViewModifier {
        var radius: CGFloat = PopupMetrics.cardRadius
        @Environment(\.headroomGlass) private var glass

        func body(content: Content) -> some View {
            content.background(
                Color.primary.opacity(glass ? 0.05 : 0.03),
                in: RoundedRectangle(cornerRadius: radius, style: .continuous))
        }
    }

    struct FooterSurface: ViewModifier {
        @Environment(\.headroomGlass) private var glass

        func body(content: Content) -> some View {
            content
                .background(glass ? Color.clear : Color.primary.opacity(0.035))
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
