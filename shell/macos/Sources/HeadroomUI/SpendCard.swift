#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct SpendSection<Trailing: View>: View {
        let spend: Spend
        let formatter: DisplayFormatter
        @Binding var period: SpendPeriod
        @ViewBuilder let trailing: () -> Trailing

        private var model: SpendCardModel {
            SpendCardModel.make(spend: spend, period: period, formatter: formatter)
        }

        var body: some View {
            let card = model
            VStack(alignment: .leading, spacing: PopupMetrics.headerGap) {
                HStack(spacing: 5) {
                    Text(formatter.strings.text(.totalSpend)).font(Typeface.title)
                    Image(systemName: "info.circle")
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary)
                        .hoverTip(id: "spend.info", text: card.info)
                    Spacer(minLength: 0)
                    trailing()
                }
                .padding(.leading, PopupMetrics.headerLeading)
                .padding(.trailing, PopupMetrics.headerTrailing)
                .frame(minHeight: PopupMetrics.refreshSize)
                VStack(spacing: 12) {
                    PeriodPicker(period: $period, strings: formatter.strings)
                    SpendBody(model: card, formatter: formatter)
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 12)
                .cardSurface()
            }
        }
    }

    struct PeriodPicker: View {
        @Binding var period: SpendPeriod
        let strings: UIStrings
        @Namespace private var selection
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            HStack(spacing: 2) {
                ForEach(SpendPeriod.allCases) { option in
                    segment(option)
                }
            }
            .padding(3)
            .background(Palette.segmentTrack, in: Capsule())
        }

        private func segment(_ option: SpendPeriod) -> some View {
            let selected = option == period
            return Button {
                Motion.perform(Motion.standard, reduced: reducedMotion) { period = option }
            } label: {
                Text(option.title(strings))
                    .font(selected ? Typeface.captionStrong : Typeface.captionMedium)
                    .foregroundStyle(selected ? .primary : .secondary)
                    .lineLimit(1)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 4)
                    .background {
                        if selected {
                            Capsule()
                                .fill(Palette.tray)
                                .shadow(color: .black.opacity(0.12), radius: 1, y: 1)
                                .matchedGeometryEffect(id: "selected", in: selection)
                        }
                    }
                    .contentShape(Capsule())
            }
            .buttonStyle(.plain)
            .accessibilityAddTraits(selected ? .isSelected : [])
        }
    }

    struct SpendBody: View {
        let model: SpendCardModel
        let formatter: DisplayFormatter

        var body: some View {
            if model.isEmpty {
                Text(formatter.strings.text(.noUsageInPeriod))
                    .font(Typeface.body)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 18)
            } else {
                HStack(spacing: PopupMetrics.legendGap) {
                    DonutView(model: model)
                    VStack(alignment: .leading, spacing: PopupMetrics.legendRowGap) {
                        ForEach(model.entries) { entry in
                            LegendRow(entry: entry)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
    }

    struct LegendRow: View {
        let entry: LegendEntry
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            VStack(alignment: .leading, spacing: 1) {
                HStack(spacing: 6) {
                    Circle().fill(entry.color.color).frame(width: 8, height: 8)
                    Text(entry.name).font(Typeface.body).lineLimit(1)
                    Spacer(minLength: 6)
                    Text(entry.amount)
                        .font(Typeface.bodyMedium)
                        .contentTransition(.numericText())
                }
                if let tokens = entry.tokensLine {
                    Text(tokens)
                        .font(Typeface.caption)
                        .foregroundStyle(.secondary)
                        .padding(.leading, 14)
                        .contentTransition(.numericText())
                }
            }
            .monospacedDigit()
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: entry.costMicros)
            .contentShape(Rectangle())
            .hoverChip(horizontal: 6, vertical: 3)
            .hoverTip(id: "legend.\(entry.id)", entry.tip)
        }
    }
#endif
