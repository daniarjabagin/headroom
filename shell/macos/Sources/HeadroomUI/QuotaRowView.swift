#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct QuotaRowView: View {
        let scope: String
        let row: QuotaRowModel
        let resetsAt: Timestamp?
        let context: PopupContext
        let now: Timestamp
        @Environment(\.headroomReducedMotion) private var reducedMotion
        @Environment(\.popupLayout) private var layout

        var body: some View {
            Group {
                if layout.isCompact {
                    compact
                } else {
                    regular
                }
            }
            .monospacedDigit()
            .padding(.horizontal, PopupMetrics.rowInset)
            .padding(.vertical, layout.cg.barRowPadding)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: row.percent)
        }

        private var regular: some View {
            VStack(alignment: .leading, spacing: PopupMetrics.rowSpacing) {
                HStack(spacing: 8) {
                    Text(row.label).font(layout.type.label).lineLimit(1)
                    Spacer(minLength: 8)
                    if let note = row.note { PaceNoteView(note: note) }
                }
                PillMeter(fraction: row.fill, tick: row.tick, tone: row.tone)
                HStack(spacing: 8) {
                    headline(font: layout.type.body)
                    Spacer(minLength: 8)
                    trailing(row.trailing, font: layout.type.reading)
                }
                if let forecast = row.forecast {
                    Text(forecast)
                        .font(Typeface.caption)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                        .padding(.top, 1)
                }
            }
        }

        private var compact: some View {
            let line = CompactLimitLine.make(
                row, resetsAt: resetsAt, display: context.display, now: now, formatter: context.formatter)
            return VStack(alignment: .leading, spacing: PopupMetrics.rowSpacing) {
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Text(row.label).font(layout.type.label).lineLimit(1)
                    Spacer(minLength: 6)
                    if line.flame {
                        Image(systemName: "flame.fill").font(.system(size: 9)).foregroundStyle(Palette.crit)
                    }
                    headline(font: layout.type.body).layoutPriority(1)
                    trailing(line.trailing, font: layout.type.body).layoutPriority(1)
                }
                PillMeter(fraction: row.fill, tick: row.tick, tone: row.tone)
                    .contentShape(Rectangle())
                    .hoverTip(id: "meter.\(scope).\(row.id)", text: line.meterTip)
            }
        }

        private func headline(font: Font) -> some View {
            ReadingButton(
                text: row.headline, font: font, style: .primary, tipID: "value.\(scope).\(row.id)",
                tip: context.valueTip, action: { context.toggleValueMode() }
            )
            .contentTransition(.opacity)
        }

        private func trailing(_ text: String, font: Font) -> some View {
            ReadingButton(
                text: text, font: font, style: .secondary, tipID: "reset.\(scope).\(row.id)", tip: context.resetTip,
                action: { context.toggleResetFormat() }
            )
            .contentTransition(.opacity)
        }
    }

    struct ReadingButton: View {
        let text: String
        let font: Font
        let style: HierarchicalShapeStyle
        let tipID: String
        let tip: String
        let action: @MainActor () -> Void

        var body: some View {
            Button {
                action()
            } label: {
                Text(text).font(font).foregroundStyle(style).lineLimit(1)
            }
            .buttonStyle(TintButtonStyle(insets: EdgeInsets(top: 1, leading: 5, bottom: 1, trailing: 5)))
            .padding(EdgeInsets(top: -1, leading: -5, bottom: -1, trailing: -5))
            .hoverTip(id: tipID, text: tip)
        }
    }

    struct PaceNoteView: View {
        let note: PaceNote

        var body: some View {
            HStack(spacing: 4) {
                if note.flame {
                    Image(systemName: "flame.fill").font(.system(size: 10)).foregroundStyle(Palette.crit)
                }
                Text(note.text).font(Typeface.caption).foregroundStyle(.secondary).lineLimit(1)
            }
        }
    }

    struct PillMeter: View {
        let fraction: Double
        let tick: Double?
        let tone: Tone
        @Environment(\.headroomReducedMotion) private var reducedMotion
        @Environment(\.popupLayout) private var layout

        var body: some View {
            let meter = layout.cg.meterHeight
            let tickHeight = layout.cg.tickHeight
            GeometryReader { proxy in
                let width = proxy.size.width
                ZStack(alignment: .leading) {
                    Capsule().fill(Palette.track).frame(height: meter)
                    Capsule()
                        .fill(tone.color)
                        .frame(width: fillWidth(in: width, meter: meter), height: meter)
                    if let tick {
                        RoundedRectangle(cornerRadius: 1)
                            .fill(Palette.tick)
                            .frame(width: PopupMetrics.tickWidth, height: tickHeight)
                            .offset(x: tickOffset(tick, in: width))
                    }
                }
                .frame(height: tickHeight)
            }
            .frame(height: tickHeight)
            .padding(.vertical, (meter - tickHeight) / 2)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: fraction)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: tick)
            .accessibilityHidden(true)
        }

        private func fillWidth(in width: CGFloat, meter: CGFloat) -> CGFloat {
            let clamped = min(1, max(0, fraction))
            return clamped == 0 ? 0 : max(meter, width * CGFloat(clamped))
        }

        private func tickOffset(_ tick: Double, in width: CGFloat) -> CGFloat {
            let position = width * CGFloat(min(1, max(0, tick))) - PopupMetrics.tickWidth / 2
            return min(max(0, position), max(0, width - PopupMetrics.tickWidth))
        }
    }
#endif
