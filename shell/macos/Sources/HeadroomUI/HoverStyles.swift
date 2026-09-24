#if canImport(AppKit)
    import SwiftUI

    struct TintButtonStyle: ButtonStyle {
        var cornerRadius: CGFloat = PopupMetrics.chipRadius
        var insets = EdgeInsets()
        var circle = false

        func makeBody(configuration: Configuration) -> some View {
            TintButtonBody(
                label: configuration.label, isPressed: configuration.isPressed, cornerRadius: cornerRadius,
                insets: insets, circle: circle)
        }
    }

    private struct TintButtonBody<Label: View>: View {
        let label: Label
        let isPressed: Bool
        let cornerRadius: CGFloat
        let insets: EdgeInsets
        let circle: Bool
        @State private var hovered = false
        @Environment(\.isEnabled) private var isEnabled
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            label
                .padding(insets)
                .background(tint, in: shape)
                .contentShape(shape)
                .onHover { hovered = $0 }
                .animation(Motion.animation(Motion.hover, reduced: reducedMotion), value: hovered)
                .animation(Motion.animation(Motion.fast, reduced: reducedMotion), value: isPressed)
        }

        private var shape: AnyShape {
            circle ? AnyShape(Circle()) : AnyShape(RoundedRectangle(cornerRadius: cornerRadius, style: .continuous))
        }

        private var tint: Color {
            guard isEnabled else { return .clear }
            if isPressed { return Palette.pressed }
            return hovered ? Palette.hover : .clear
        }
    }

    struct HoverChip: ViewModifier {
        var horizontal: CGFloat = 5
        var vertical: CGFloat = 2
        @State private var hovered = false
        @Environment(\.headroomReducedMotion) private var reducedMotion

        func body(content: Content) -> some View {
            content
                .background(
                    RoundedRectangle(cornerRadius: PopupMetrics.chipRadius, style: .continuous)
                        .fill(hovered ? Palette.hover : .clear)
                        .padding(
                            EdgeInsets(top: -vertical, leading: -horizontal, bottom: -vertical, trailing: -horizontal))
                )
                .onHover { hovered = $0 }
                .animation(Motion.animation(Motion.hover, reduced: reducedMotion), value: hovered)
        }
    }

    extension View {
        func hoverChip(horizontal: CGFloat = 5, vertical: CGFloat = 2) -> some View {
            modifier(HoverChip(horizontal: horizontal, vertical: vertical))
        }
    }
#endif
