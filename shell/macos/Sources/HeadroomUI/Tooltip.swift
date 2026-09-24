#if canImport(AppKit)
    import HeadroomKit
    import Observation
    import SwiftUI

    enum PopupSpace {
        static let root = "headroom.popup"
        static let accounts = "headroom.accounts"
    }

    struct ShownTip: Equatable {
        let id: String
        let content: TipContent
        let anchor: CGRect
    }

    @MainActor
    @Observable
    final class TipCenter {
        private static let warmWindow: TimeInterval = 0.3

        private(set) var shown: ShownTip?
        private var pending: Task<Void, Never>?
        private var pendingID: String?
        private var hiddenAt = Date.distantPast

        func hover(_ active: Bool, id: String, anchor: CGRect, content: TipContent?) {
            if active, let content {
                schedule(ShownTip(id: id, content: content, anchor: anchor))
            } else if pendingID == id || shown?.id == id {
                hide()
            }
        }

        func hide() {
            pending?.cancel()
            pending = nil
            pendingID = nil
            if shown != nil { hiddenAt = Date() }
            shown = nil
        }

        private func schedule(_ tip: ShownTip) {
            pending?.cancel()
            pendingID = tip.id
            if shown != nil || Date().timeIntervalSince(hiddenAt) < Self.warmWindow {
                shown = tip
                return
            }
            pending = Task { [weak self] in
                try? await Task.sleep(for: PopupMetrics.tipDelay)
                guard !Task.isCancelled else { return }
                self?.shown = tip
            }
        }
    }

    struct HoverTip: ViewModifier {
        let id: String
        let content: TipContent?
        @Environment(TipCenter.self) private var tips: TipCenter?
        @State private var frame = CGRect.zero

        func body(content view: Content) -> some View {
            view
                .background {
                    GeometryReader { proxy in
                        Color.clear.onChange(of: proxy.frame(in: .named(PopupSpace.root)), initial: true) { _, new in
                            frame = new
                        }
                    }
                }
                .onHover { hovering in
                    tips?.hover(hovering, id: id, anchor: frame, content: content)
                }
        }
    }

    extension View {
        func hoverTip(id: String, _ content: TipContent?) -> some View {
            modifier(HoverTip(id: id, content: content))
        }

        func hoverTip(id: String, text: String?) -> some View {
            modifier(HoverTip(id: id, content: text.map(TipContent.text)))
        }
    }

    struct TipOverlay: View {
        let center: TipCenter
        @State private var size = CGSize.zero
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            GeometryReader { proxy in
                if let tip = center.shown {
                    TipBubble(content: tip.content)
                        .fixedSize()
                        .background {
                            GeometryReader { bubble in
                                Color.clear.onChange(of: bubble.size, initial: true) { _, new in size = new }
                            }
                        }
                        .offset(origin(for: tip.anchor, in: proxy.size))
                        .opacity(size == .zero ? 0 : 1)
                        .transition(.opacity)
                        .id(tip.id)
                }
            }
            .allowsHitTesting(false)
            .animation(Motion.animation(Motion.fast, reduced: reducedMotion), value: center.shown?.id)
        }

        private func origin(for anchor: CGRect, in bounds: CGSize) -> CGSize {
            let gap: CGFloat = 6
            let margin: CGFloat = 8
            let x = min(max(anchor.midX - size.width / 2, margin), max(margin, bounds.width - size.width - margin))
            let below = anchor.maxY + gap
            let fitsBelow = below + size.height <= bounds.height - margin
            let y = fitsBelow ? below : max(margin, anchor.minY - gap - size.height)
            return CGSize(width: x, height: y)
        }
    }

    struct TipBubble: View {
        let content: TipContent

        var body: some View {
            Group {
                switch content {
                case .text(let text):
                    Text(text).font(Typeface.caption).frame(maxWidth: 220, alignment: .leading)
                case .day(let title, let detail):
                    VStack(alignment: .leading, spacing: 2) {
                        Text(title).font(Typeface.captionStrong)
                        Text(detail).font(Typeface.caption).foregroundStyle(.secondary)
                    }
                case .breakdown(let breakdown):
                    BreakdownTip(breakdown: breakdown)
                }
            }
            .monospacedDigit()
            .fixedSize(horizontal: false, vertical: true)
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
            .background(.regularMaterial, in: RoundedRectangle(cornerRadius: PopupMetrics.chipRadius, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: PopupMetrics.chipRadius, style: .continuous)
                    .strokeBorder(Palette.separator, lineWidth: 0.5)
            )
            .shadow(color: .black.opacity(0.18), radius: 9, y: 4)
        }
    }

    struct BreakdownTip: View {
        let breakdown: ModelBreakdown

        var body: some View {
            VStack(alignment: .leading, spacing: 5) {
                Text(breakdown.title).font(Typeface.captionStrong)
                Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 2) {
                    ForEach(Array(breakdown.rows.enumerated()), id: \.offset) { _, row in
                        GridRow {
                            Text(row.name).lineLimit(1).truncationMode(.middle).frame(maxWidth: 140, alignment: .leading)
                            Text(row.tokens).foregroundStyle(.secondary).gridColumnAlignment(.trailing)
                            Text(row.cost).gridColumnAlignment(.trailing)
                        }
                    }
                }
                .font(Typeface.caption)
                Rectangle().fill(Palette.separator).frame(height: 1)
                Text(breakdown.total).font(Typeface.captionMedium)
                if let note = breakdown.partialNote {
                    Text(note).font(Typeface.caption2).foregroundStyle(.secondary)
                }
            }
        }
    }
#endif
