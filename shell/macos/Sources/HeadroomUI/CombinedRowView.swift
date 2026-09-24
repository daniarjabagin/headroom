#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct CombinedLimitsView: View {
        let sectionID: String
        let limits: CombinedLimits
        let context: PopupContext
        let now: Timestamp

        var body: some View {
            ForEach(limits.notices) { notice in NoticeRow(notice: notice, context: context) }
            ForEach(limits.windows) { window in
                CombinedRowView(
                    tipID: "combined.\(sectionID).\(window.id)",
                    row: CombinedRowModel.make(
                        window, display: context.display, now: now, formatter: context.formatter),
                    toggleValueMode: { context.toggleValueMode() },
                    toggleResetFormat: { context.toggleResetFormat() })
            }
        }
    }

    struct CombinedRowView: View {
        let tipID: String
        let row: CombinedRowModel
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
                SegmentedPillMeter(segments: row.segments)
                HStack(spacing: 8) {
                    ReadingButton(text: row.headline, font: Typeface.body, style: .primary, action: toggleValueMode)
                        .contentTransition(.numericText())
                    Spacer(minLength: 8)
                    ReadingButton(
                        text: row.trailing, font: Typeface.caption, style: .secondary, action: toggleResetFormat)
                }
                if let forecast = row.forecast {
                    Text(forecast)
                        .font(Typeface.caption)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                        .padding(.top, 1)
                }
            }
            .monospacedDigit()
            .padding(.horizontal, PopupMetrics.rowInset)
            .padding(.vertical, PopupMetrics.barRowPadding)
            .contentShape(Rectangle())
            .hoverTip(id: tipID, row.tip)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: row.percent)
        }
    }

    struct SegmentedPillMeter: View {
        let segments: [SegmentModel]
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            GeometryReader { proxy in
                let spans = SegmentedMeter.spans(count: segments.count, width: Double(proxy.size.width))
                ZStack(alignment: .leading) {
                    ForEach(Array(zip(segments, spans).enumerated()), id: \.offset) { _, pair in
                        segmentView(pair.0, span: pair.1)
                    }
                }
            }
            .frame(height: PopupMetrics.meterHeight)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: segments)
            .accessibilityHidden(true)
        }

        private func segmentView(_ segment: SegmentModel, span: MeterSpan) -> some View {
            let fill = SegmentedMeter.fillWidth(
                segment.fill, span: span.width, minimum: Double(PopupMetrics.meterHeight))
            return ZStack(alignment: .leading) {
                Capsule().fill(Palette.track)
                Capsule().fill(segment.tone.color).frame(width: CGFloat(fill))
            }
            .frame(width: CGFloat(span.width), height: PopupMetrics.meterHeight)
            .offset(x: CGFloat(span.offset))
        }
    }
#endif
