#if canImport(AppKit)
    import AppKit
    import SwiftUI

    struct PopupSurface: ViewModifier {
        let translucent: Bool

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
                    .background(Color(nsColor: .textBackgroundColor), in: shape)
                    .overlay(shape.strokeBorder(Color.primary.opacity(0.12), lineWidth: 1))
            }
        }
    }

    struct CardSurface: ViewModifier {
        private var shape: RoundedRectangle {
            RoundedRectangle(cornerRadius: PopupMetrics.cardRadius, style: .continuous)
        }

        @ViewBuilder
        func body(content: Content) -> some View {
            content.background(Color.primary.opacity(0.03), in: shape)
        }
    }

    struct ActionButtonStyle: ViewModifier {
        @ViewBuilder
        func body(content: Content) -> some View {
            #if compiler(>=6.2)
                if #available(macOS 26, *) {
                    content.buttonStyle(.glass)
                } else {
                    content.buttonStyle(.bordered)
                }
            #else
                content.buttonStyle(.bordered)
            #endif
        }
    }

    struct ActionGroup<Content: View>: View {
        @ViewBuilder let content: () -> Content

        @ViewBuilder
        var body: some View {
            #if compiler(>=6.2)
                if #available(macOS 26, *) {
                    GlassEffectContainer { HStack(spacing: 8, content: content) }
                } else {
                    HStack(spacing: 8, content: content)
                }
            #else
                HStack(spacing: 8, content: content)
            #endif
        }
    }

    extension View {
        func popupSurface(translucent: Bool) -> some View {
            modifier(PopupSurface(translucent: translucent))
        }

        func cardSurface() -> some View {
            modifier(CardSurface())
        }

        func actionButtonStyle() -> some View {
            modifier(ActionButtonStyle())
        }
    }
#endif
