#if canImport(AppKit)
    import Foundation
    import HeadroomKit
    import SwiftUI

    struct FooterView: View {
        let screen: PopupScreen
        let formatter: DisplayFormatter
        let refresh: @MainActor () -> Void
        let openSettings: @MainActor () -> Void

        private var versionLine: String {
            let version = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String
            return version.map { formatter.strings.fill(.version, ["version": $0]) } ?? "Headroom"
        }

        var body: some View {
            HStack(spacing: 8) {
                VStack(alignment: .leading, spacing: 1) {
                    Text(versionLine)
                    TimelineView(.periodic(from: .now, by: 1)) { context in
                        statusLine(FooterStatus.make(screen: screen, now: Timestamp(date: context.date), formatter: formatter))
                    }
                }
                .font(Typeface.caption2)
                .foregroundStyle(.secondary)
                .monospacedDigit()
                Spacer(minLength: 8)
                Button {
                    openSettings()
                } label: {
                    Image(systemName: "gearshape")
                        .font(.system(size: 15, weight: .regular))
                        .foregroundStyle(.secondary)
                        .frame(width: PopupMetrics.gearSize, height: PopupMetrics.gearSize)
                }
                .buttonStyle(TintButtonStyle(circle: true))
                .hoverTip(id: "settings", text: formatter.strings.text(.settings))
                .accessibilityLabel(formatter.strings.text(.settings))
            }
            .padding(.horizontal, PopupMetrics.padding)
            .padding(.vertical, 12)
            .footerSurface()
        }

        @ViewBuilder
        private func statusLine(_ status: FooterStatus) -> some View {
            let text = Text(status.text).foregroundStyle(status.isNotice ? AnyShapeStyle(Palette.notice) : AnyShapeStyle(.secondary))
            if status.refreshes && !status.text.isEmpty {
                Button {
                    refresh()
                } label: {
                    text
                }
                .buttonStyle(.plain)
                .hoverChip(horizontal: 4, vertical: 1)
            } else {
                text
            }
        }
    }
#endif
