#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct MoreRow: View {
        let summary: FoldSummary
        let expand: @MainActor () -> Void
        @State private var hovered = false
        @Environment(\.headroomReducedMotion) private var reducedMotion
        @Environment(\.popupLayout) private var layout

        var body: some View {
            Button {
                expand()
            } label: {
                HStack(spacing: 8) {
                    HStack(spacing: 4) {
                        ForEach(summary.providers, id: \.self) { provider in
                            ProviderGlyph(provider: provider, size: layout.cg.glyphSize)
                        }
                    }
                    Text(summary.title).font(layout.type.bodyStrong).lineLimit(1).layoutPriority(1)
                    Text(summary.names)
                        .font(layout.type.body)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .truncationMode(.tail)
                    Spacer(minLength: 0)
                    attentionMark
                    Image(systemName: "chevron.right")
                        .font(.system(size: 10, weight: .semibold))
                        .foregroundStyle(.secondary)
                }
                .padding(.leading, 14)
                .padding(.trailing, 12)
                .padding(.vertical, layout.isCompact ? 7 : 10)
                .background {
                    RoundedRectangle(cornerRadius: PopupMetrics.cardRadius, style: .continuous)
                        .fill(hovered ? Palette.hover : .clear)
                }
                .cardSurface()
                .contentShape(RoundedRectangle(cornerRadius: PopupMetrics.cardRadius, style: .continuous))
            }
            .buttonStyle(.plain)
            .accessibilityLabel(Text(verbatim: summary.accessibilityLabel))
            .onHover { hovered = $0 }
            .animation(Motion.animation(Motion.hover, reduced: reducedMotion), value: hovered)
        }

        @ViewBuilder
        private var attentionMark: some View {
            switch summary.attention.kind {
            case .notice:
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(summary.attention.tone == .critical ? Palette.crit : Palette.notice)
                    .hoverTip(id: "fold.attention", text: summary.attentionText)
            case .tone:
                Circle()
                    .fill(summary.attention.tone.color)
                    .frame(width: 6, height: 6)
                    .padding(.horizontal, 2)
                    .hoverTip(id: "fold.attention", text: summary.attentionText)
            case .none:
                EmptyView()
            }
        }
    }

    struct NotPinnedDivider: View {
        let strings: UIStrings
        let collapse: @MainActor () -> Void

        var body: some View {
            HStack(spacing: 8) {
                Text(strings.text(PopupExtraText.notPinned))
                    .font(Typeface.captionStrong)
                    .foregroundStyle(.secondary)
                Rectangle().fill(Palette.separator).frame(height: 1)
                Button {
                    collapse()
                } label: {
                    HStack(spacing: 3) {
                        Text(strings.text(PopupExtraText.showLess)).font(Typeface.captionMedium)
                        Image(systemName: "chevron.up").font(.system(size: 9, weight: .semibold))
                    }
                    .foregroundStyle(.secondary)
                }
                .buttonStyle(TintButtonStyle(insets: EdgeInsets(top: 2, leading: 6, bottom: 2, trailing: 6)))
            }
            .padding(.leading, PopupMetrics.headerLeading)
            .padding(.trailing, PopupMetrics.headerTrailing)
        }
    }

    struct ToastHost: View {
        let ui: PopupUIState
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            ZStack {
                if let toast = ui.toast {
                    ToastView(toast: toast)
                        .id(toast.id)
                        .transition(.opacity.combined(with: .offset(y: 8)))
                        .task(id: toast.id) { await dismiss(toast.id) }
                }
            }
            .padding(.bottom, 14)
            .allowsHitTesting(false)
        }

        private func dismiss(_ id: Int) async {
            try? await Task.sleep(for: .seconds(PopupToast.visibleSeconds))
            guard !Task.isCancelled, ui.toast?.id == id else { return }
            Motion.perform(Motion.fast, reduced: reducedMotion) { ui.toast = nil }
        }
    }

    struct ToastView: View {
        let toast: PopupToast

        var body: some View {
            HStack(spacing: 6) {
                Image(systemName: toast.kind == .success ? "checkmark.circle.fill" : "xmark.octagon.fill")
                    .font(.system(size: 13))
                    .foregroundStyle(toast.kind == .success ? Color(nsColor: .systemGreen) : Palette.crit)
                Text(toast.title).font(Typeface.captionStrong)
                if let detail = toast.detail {
                    Text(verbatim: "· \(detail)").font(Typeface.caption).foregroundStyle(.secondary)
                }
            }
            .lineLimit(1)
            .padding(EdgeInsets(top: 6, leading: 9, bottom: 6, trailing: 12))
            .background(Palette.tooltip, in: Capsule())
            .overlay(Capsule().strokeBorder(Palette.separator, lineWidth: 0.5))
            .shadow(color: .black.opacity(0.18), radius: 10, y: 4)
        }
    }
#endif
