#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct AccountHeaderView: View {
        let header: AccountHeaderModel
        let context: PopupContext
        let now: Timestamp
        let reorder: ReorderHandle?
        @State private var hovered = false
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            HStack(spacing: 5) {
                ProviderGlyph(provider: header.provider)
                HStack(alignment: .firstTextBaseline, spacing: 5) {
                    Text(header.title)
                        .font(Typeface.title)
                        .lineLimit(1)
                        .truncationMode(.tail)
                        .layoutPriority(1)
                    if let plan = header.plan {
                        Text(plan).font(Typeface.caption).foregroundStyle(.secondary).lineLimit(1).layoutPriority(1)
                    }
                    if case .outdated(let updatedAt) = header.status {
                        Text(context.strings.text(.outdated))
                            .font(Typeface.caption)
                            .foregroundStyle(.tertiary)
                            .lineLimit(1)
                            .hoverTip(id: "outdated.\(header.title)", text: outdatedTip(updatedAt))
                    }
                }
                statusGlyph
                Spacer(minLength: 0)
                Image(systemName: "line.3.horizontal")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(.tertiary)
                    .opacity(hovered && reorder != nil ? 1 : 0)
                    .accessibilityHidden(true)
            }
            .padding(.leading, PopupMetrics.headerLeading)
            .padding(.trailing, PopupMetrics.headerTrailing)
            .padding(.vertical, 2)
            .contentShape(Rectangle())
            .onHover { hovered = $0 }
            .animation(Motion.animation(Motion.fast, reduced: reducedMotion), value: hovered)
            .gesture(dragGesture, including: reorder == nil ? .subviews : .all)
        }

        private var dragGesture: some Gesture {
            DragGesture(minimumDistance: 6, coordinateSpace: .named(PopupSpace.accounts))
                .onChanged { value in reorder?.changed(value) }
                .onEnded { _ in reorder?.ended() }
        }

        @ViewBuilder
        private var statusGlyph: some View {
            switch header.status {
            case .refreshing:
                BusyIndicator(size: 12)
            case .failed(let message):
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(Palette.notice)
                    .hoverTip(
                        id: "failed.\(header.title)",
                        text: message.isEmpty ? context.strings.text(.refreshFailed) : message)
            case .outdated, .none:
                EmptyView()
            }
        }

        private func outdatedTip(_ updatedAt: Timestamp?) -> String? {
            guard let updatedAt else { return nil }
            return context.strings.fill(.lastUpdated, ["ago": context.formatter.agoText(updatedAt, now: now)])
        }
    }
#endif
