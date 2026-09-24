#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct QuotaRowView: View {
        let row: QuotaRowModel
        let toggleValueMode: @MainActor () -> Void
        let toggleResetFormat: @MainActor () -> Void
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            VStack(alignment: .leading, spacing: PopupMetrics.rowSpacing) {
                HStack(spacing: 8) {
                    Text(row.label).font(Typeface.label).lineLimit(1)
                    Spacer(minLength: 8)
                    if let note = row.note { PaceNoteView(note: note) }
                }
                PillMeter(fraction: row.fill, tick: row.tick, tone: row.tone)
                HStack(spacing: 8) {
                    toggle(row.headline, style: .primary, action: toggleValueMode)
                        .contentTransition(.numericText())
                    Spacer(minLength: 8)
                    toggle(row.trailing, style: .secondary, action: toggleResetFormat)
                }
                if let forecast = row.forecast {
                    Text(forecast)
                        .font(Typeface.caption)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                        .padding(.top, 2)
                }
            }
            .monospacedDigit()
            .padding(.horizontal, PopupMetrics.rowInset)
            .padding(.vertical, PopupMetrics.barRowPadding)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: row.percent)
        }

        private func toggle(_ text: String, style: HierarchicalShapeStyle, action: @escaping @MainActor () -> Void)
            -> some View
        {
            Button {
                action()
            } label: {
                Text(text).font(Typeface.body).foregroundStyle(style).lineLimit(1)
            }
            .buttonStyle(TintButtonStyle(insets: EdgeInsets(top: 1, leading: 5, bottom: 1, trailing: 5)))
            .padding(EdgeInsets(top: -1, leading: -5, bottom: -1, trailing: -5))
        }
    }

    struct PaceNoteView: View {
        let note: PaceNote

        var body: some View {
            HStack(spacing: 4) {
                if note.flame {
                    Image(systemName: "flame.fill").font(.system(size: 11)).foregroundStyle(Palette.crit)
                }
                Text(note.text).font(Typeface.body).foregroundStyle(.secondary).lineLimit(1)
            }
        }
    }

    struct PillMeter: View {
        let fraction: Double
        let tick: Double?
        let tone: Tone
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            GeometryReader { proxy in
                let width = proxy.size.width
                ZStack(alignment: .leading) {
                    Capsule().fill(Palette.track).frame(height: PopupMetrics.meterHeight)
                    Capsule()
                        .fill(tone.color)
                        .frame(width: fillWidth(in: width), height: PopupMetrics.meterHeight)
                    if let tick {
                        RoundedRectangle(cornerRadius: 1)
                            .fill(Palette.tick)
                            .frame(width: PopupMetrics.tickWidth, height: PopupMetrics.tickHeight)
                            .offset(x: tickOffset(tick, in: width))
                    }
                }
                .frame(height: PopupMetrics.tickHeight)
            }
            .frame(height: PopupMetrics.tickHeight)
            .padding(.vertical, (PopupMetrics.meterHeight - PopupMetrics.tickHeight) / 2)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: fraction)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: tick)
            .accessibilityHidden(true)
        }

        private func fillWidth(in width: CGFloat) -> CGFloat {
            let clamped = min(1, max(0, fraction))
            return clamped == 0 ? 0 : max(PopupMetrics.meterHeight, width * CGFloat(clamped))
        }

        private func tickOffset(_ tick: Double, in width: CGFloat) -> CGFloat {
            let position = width * CGFloat(min(1, max(0, tick))) - PopupMetrics.tickWidth / 2
            return min(max(0, position), max(0, width - PopupMetrics.tickWidth))
        }
    }
#endif
