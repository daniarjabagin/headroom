#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct TrendRow: View {
        let accountID: String
        let bars: [TrendBar]
        let title: String

        var body: some View {
            HStack(spacing: 10) {
                Text(title).font(Typeface.bodyStrong).lineLimit(1)
                Spacer(minLength: 12)
                HStack(alignment: .bottom, spacing: 1) {
                    ForEach(bars) { bar in
                        TrendBarView(accountID: accountID, bar: bar)
                    }
                }
                .frame(
                    minWidth: 90, maxWidth: 150, minHeight: CGFloat(UsageRows.trendHeight),
                    maxHeight: CGFloat(UsageRows.trendHeight))
            }
            .padding(.horizontal, PopupMetrics.rowInset)
            .padding(.vertical, PopupMetrics.textRowPadding)
        }
    }

    private struct TrendBarView: View {
        let accountID: String
        let bar: TrendBar
        @State private var hovered = false

        var body: some View {
            VStack(spacing: 0) {
                Spacer(minLength: 0)
                UnevenRoundedRectangle(topLeadingRadius: 1, topTrailingRadius: 1, style: .continuous)
                    .fill(Palette.ok.opacity(hovered ? 0.75 : 1))
                    .frame(height: CGFloat(bar.height))
            }
            .frame(minWidth: 2, maxWidth: .infinity, maxHeight: .infinity)
            .contentShape(Rectangle())
            .onHover { hovered = $0 && bar.tip != nil }
            .hoverTip(id: "trend.\(accountID).\(bar.id)", bar.tip)
        }
    }

    struct ValueRow: View {
        let accountID: String
        let row: ValueRowModel

        var body: some View {
            HStack(spacing: 10) {
                Text(row.title).font(Typeface.bodyStrong).lineLimit(1)
                Spacer(minLength: 12)
                value
            }
            .padding(.horizontal, PopupMetrics.rowInset)
            .padding(.vertical, 4)
        }

        @ViewBuilder
        private var value: some View {
            let text = Text(row.value).font(Typeface.body).monospacedDigit().lineLimit(1)
            if let tip = row.tip {
                text.hoverChip().hoverTip(id: "value.\(accountID).\(row.id)", tip)
            } else {
                text
            }
        }
    }

    struct Expander: View {
        let expanded: Bool
        let toggle: @MainActor () -> Void
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            Button {
                toggle()
            } label: {
                Image(systemName: "chevron.down")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(.secondary)
                    .rotationEffect(.degrees(expanded ? 180 : 0))
                    .frame(width: 14, height: 14)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 5)
            }
            .buttonStyle(TintButtonStyle())
            .padding(.horizontal, 8)
            .animation(Motion.animation(Motion.toggle, reduced: reducedMotion), value: expanded)
        }
    }
#endif
