#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct CombinedLimitsView: View {
        let sectionID: String
        let limits: CombinedLimits
        let context: PopupContext
        let now: Timestamp
        let status: StatusNoticeModel?

        var body: some View {
            ForEach(limits.notices) { notice in NoticeRow(notice: notice, context: context) }
            if let status { StatusNoticeView(notice: status, open: { context.actions.openURL($0) }) }
            ForEach(limits.windows) { window in
                switch row(window) {
                case .single(let model):
                    QuotaRowView(scope: sectionID, row: model, resetsAt: window.resetsAt, context: context, now: now)
                case .pooled(let model):
                    CombinedRowView(scope: sectionID, row: model, context: context)
                }
            }
        }

        private func row(_ window: CombinedWindow) -> CombinedLimitRow {
            CombinedLimitRow.make(
                window, members: limits.members, display: context.display, now: now, formatter: context.formatter)
        }
    }

    struct CombinedRowView: View {
        let scope: String
        let row: CombinedRowModel
        let context: PopupContext
        @Environment(\.headroomReducedMotion) private var reducedMotion
        @Environment(\.popupLayout) private var layout

        var body: some View {
            VStack(alignment: .leading, spacing: PopupMetrics.rowSpacing) {
                HStack(spacing: 8) {
                    Text(row.label).font(layout.type.label).lineLimit(1)
                    Spacer(minLength: 8)
                    if let note = row.note { PaceNoteView(note: note) }
                }
                SegmentedPillMeter(segments: row.segments)
                HStack(spacing: 8) {
                    ReadingButton(
                        text: row.headline, font: layout.type.body, style: .primary, tipID: "value.\(scope).\(row.id)",
                        tip: context.valueTip, action: { context.toggleValueMode() }
                    )
                    .contentTransition(.opacity)
                    Spacer(minLength: 8)
                    ReadingButton(
                        text: row.trailing, font: layout.type.reading, style: .secondary,
                        tipID: "reset.\(scope).\(row.id)", tip: context.resetTip,
                        action: { context.toggleResetFormat() })
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
            .padding(.vertical, layout.cg.barRowPadding)
            .contentShape(Rectangle())
            .hoverTip(id: "combined.\(scope).\(row.id)", row.tip)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: row.percent)
        }
    }

    struct SegmentedPillMeter: View {
        let segments: [SegmentModel]
        @Environment(\.headroomReducedMotion) private var reducedMotion
        @Environment(\.popupLayout) private var layout

        private var meterHeight: CGFloat { layout.cg.meterHeight }
        private var tickHeight: CGFloat { layout.cg.tickHeight }

        var body: some View {
            GeometryReader { proxy in
                let spans = SegmentedMeter.spans(count: segments.count, width: Double(proxy.size.width))
                ZStack(alignment: .leading) {
                    ForEach(Array(zip(segments, spans).enumerated()), id: \.offset) { _, pair in
                        segmentView(pair.0, span: pair.1)
                    }
                }
                .frame(height: tickHeight)
            }
            .frame(height: tickHeight)
            .padding(.vertical, (meterHeight - tickHeight) / 2)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: segments)
            .accessibilityHidden(true)
        }

        private func segmentView(_ segment: SegmentModel, span: MeterSpan) -> some View {
            let fill = SegmentedMeter.fillWidth(
                segment.fill, span: span.width, minimum: Double(meterHeight))
            return ZStack(alignment: .leading) {
                Capsule().fill(Palette.track).frame(height: meterHeight)
                Capsule()
                    .fill(segment.tone.color)
                    .frame(width: CGFloat(fill), height: meterHeight)
                    .meterSheen(fill: segment.fill, width: CGFloat(fill), height: meterHeight)
                if let tick = segment.tick {
                    RoundedRectangle(cornerRadius: 1)
                        .fill(Palette.tick)
                        .frame(width: PopupMetrics.tickWidth, height: tickHeight)
                        .offset(x: tickOffset(tick, span: span))
                }
            }
            .frame(width: CGFloat(span.width), height: tickHeight, alignment: .leading)
            .offset(x: CGFloat(span.offset))
        }

        private func tickOffset(_ tick: Double, span: MeterSpan) -> CGFloat {
            CGFloat(
                SegmentedMeter.tickOffset(tick, span: span.width, tickWidth: Double(PopupMetrics.tickWidth)))
        }
    }
#endif
