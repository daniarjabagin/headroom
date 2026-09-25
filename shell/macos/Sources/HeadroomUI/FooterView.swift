#if canImport(AppKit)
    import AppKit
    import Foundation
    import HeadroomKit
    import SwiftUI

    struct FooterView: View {
        let screen: PopupScreen
        let formatter: DisplayFormatter
        let refresh: @MainActor () -> Void
        let openSettings: @MainActor () -> Void
        @Environment(\.popupLayout) private var layout
        @Environment(\.headroomReducedMotion) private var reducedMotion

        private var versionLine: String {
            let version = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String
            return version.map { formatter.strings.fill(.version, ["version": $0]) } ?? "Headroom"
        }

        var body: some View {
            HStack(spacing: 8) {
                TimelineView(.periodic(from: .now, by: 1)) { context in
                    lines(
                        FooterModel.make(
                            screen: screen, now: Timestamp(date: context.date), formatter: formatter,
                            version: versionLine))
                }
                .font(Typeface.caption2)
                .monospacedDigit()
                Spacer(minLength: 8)
                settingsButton
            }
            .padding(.horizontal, PopupMetrics.padding)
            .padding(.vertical, layout.cg.footerPaddingY)
            .footerSurface()
        }

        private var settingsButton: some View {
            let tip = formatter.strings.fill(
                PopupExtraText.settingsVersion, ["settings": formatter.strings.text(.settings), "version": versionLine])
            return Button {
                openSettings()
            } label: {
                Image(systemName: "gearshape")
                    .font(.system(size: 14, weight: .regular))
                    .foregroundStyle(.secondary)
                    .frame(width: layout.cg.gearSize, height: layout.cg.gearSize)
            }
            .buttonStyle(TintButtonStyle(circle: true))
            .hoverTip(id: "settings", text: tip)
            .accessibilityLabel(formatter.strings.text(.settings))
        }

        private func lines(_ model: FooterModel) -> some View {
            let stale = model.kind == .stale
            let style = stale ? AnyShapeStyle(Palette.notice) : AnyShapeStyle(.secondary)
            return VStack(alignment: .leading, spacing: 1) {
                HStack(spacing: 4) {
                    if stale {
                        Image(systemName: "exclamationmark.triangle.fill").font(.system(size: 9))
                    }
                    Text(model.primary)
                }
                secondLine(model)
            }
            .foregroundStyle(style)
            .lineLimit(1)
            .animation(Motion.animation(Motion.fast, reduced: reducedMotion), value: model.kind)
        }

        @ViewBuilder
        private func secondLine(_ model: FooterModel) -> some View {
            let content = HStack(spacing: 4) {
                if model.kind == .live { LiveDot() }
                Text(model.secondary)
            }
            if model.refreshes && !model.secondary.isEmpty {
                Button {
                    refresh()
                } label: {
                    content
                }
                .buttonStyle(.plain)
                .hoverChip(horizontal: 4, vertical: 1)
                .hoverTip(id: "footer.live", text: model.tip)
            } else {
                content
            }
        }
    }

    struct LiveDot: View {
        var body: some View {
            Circle()
                .fill(Color(nsColor: .systemGreen))
                .frame(width: 6, height: 6)
                .background(Circle().fill(Color(nsColor: .systemGreen).opacity(0.22)).frame(width: 12, height: 12))
                .padding(.horizontal, 3)
                .accessibilityHidden(true)
        }
    }
#endif
