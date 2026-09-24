#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct ThinScrollView<Content: View>: View {
        let height: CGFloat
        let contentHeight: CGFloat
        let content: Content
        @State private var offset: CGFloat = 0
        @State private var visible = false
        @State private var hideTask: Task<Void, Never>?
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            ScrollView(.vertical) {
                content.background { ScrollOffsetReader(offset: $offset) }
            }
            .scrollIndicators(.never)
            .coordinateSpace(.named(PopupSpace.scroll))
            .frame(height: height)
            .overlay(alignment: .topTrailing) { indicator }
            .onChange(of: offset) { _, _ in reveal() }
            .onAppear { reveal() }
            .onDisappear { hideTask?.cancel() }
        }

        @ViewBuilder
        private var indicator: some View {
            if let knob = ScrollIndicator.knob(
                viewport: Double(height), content: Double(contentHeight), offset: Double(offset))
            {
                Capsule()
                    .fill(Palette.scrollKnob)
                    .frame(width: CGFloat(ScrollIndicator.width), height: CGFloat(knob.length))
                    .offset(y: CGFloat(knob.top))
                    .padding(.trailing, CGFloat(ScrollIndicator.edgeInset))
                    .opacity(visible ? 1 : 0)
                    .allowsHitTesting(false)
                    .accessibilityHidden(true)
            }
        }

        private func reveal() {
            hideTask?.cancel()
            if !visible {
                Motion.perform(Motion.fast, reduced: reducedMotion) { visible = true }
            }
            hideTask = Task { @MainActor in
                try? await Task.sleep(for: ScrollIndicator.fadeDelay)
                guard !Task.isCancelled else { return }
                Motion.perform(Motion.fade, reduced: reducedMotion) { visible = false }
            }
        }
    }

    struct ScrollOffsetReader: View {
        @Binding var offset: CGFloat

        var body: some View {
            GeometryReader { proxy in
                Color.clear.onChange(of: proxy.frame(in: .named(PopupSpace.scroll)).minY, initial: true) { _, minY in
                    offset = -minY
                }
            }
        }
    }
#endif
