#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct SpendSection<Trailing: View>: View {
        let spend: Spend
        let formatter: DisplayFormatter
        let units: [SpendUnit]
        let showBreakdown: Bool
        @Binding var selection: SpendSelection
        @ViewBuilder let trailing: () -> Trailing
        @Environment(\.popupLayout) private var layout

        private var model: SpendCardModel {
            SpendCardModel.make(
                spend: spend, selection: selection, formatter: formatter, showBreakdown: showBreakdown)
        }

        var body: some View {
            let card = model
            VStack(alignment: .leading, spacing: layout.cg.headerGap) {
                HStack(spacing: 5) {
                    UnitTitle(title: card.title, units: units, strings: formatter.strings, unit: $selection.unit)
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
                VStack(spacing: layout.cg.spendCardPaddingY) {
                    PeriodPicker(
                        periods: SpendPeriodPreference.available(in: spend), selected: card.period,
                        strings: formatter.strings, select: { selection.period = $0 })
                    SpendBody(model: card, formatter: formatter)
                    if let breakdown = card.breakdown {
                        SpendBreakdownView(
                            model: breakdown, strings: formatter.strings, select: { selection.breakdown = $0 })
                    }
                }
                .padding(.horizontal, PopupMetrics.cardPaddingX)
                .padding(.vertical, layout.cg.spendCardPaddingY)
                .cardSurface()
            }
        }
    }

    struct UnitTitle: View {
        let title: String
        let units: [SpendUnit]
        let strings: UIStrings
        @Binding var unit: SpendUnit
        @Environment(\.popupLayout) private var layout

        var body: some View {
            if units.count > 1 {
                Menu {
                    ForEach(units) { option in
                        Toggle(isOn: Binding(get: { unit == option }, set: { if $0 { unit = option } })) {
                            Text(option.title(strings))
                        }
                        .help(option.detail(strings))
                    }
                } label: {
                    HStack(spacing: 4) {
                        Text(title).font(layout.type.title)
                        Image(systemName: "chevron.down").font(.system(size: 9, weight: .semibold))
                            .foregroundStyle(.secondary)
                    }
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .contentShape(Rectangle())
                }
                .menuStyle(.button)
                .buttonStyle(TintButtonStyle())
                .menuIndicator(.hidden)
                .fixedSize()
                .padding(.leading, -6)
            } else {
                Text(title).font(layout.type.title)
            }
        }
    }

    struct PeriodPicker: View {
        let periods: [SpendPeriodPreference]
        let selected: SpendPeriodPreference
        let strings: UIStrings
        let select: @MainActor (SpendPeriodPreference) -> Void
        @Namespace private var selection
        @Environment(\.headroomReducedMotion) private var reducedMotion
        @Environment(\.popupLayout) private var layout

        var body: some View {
            HStack(spacing: 2) {
                ForEach(periods) { option in
                    segment(option)
                }
            }
            .padding(3)
            .background(Palette.segmentTrack, in: Capsule())
        }

        private func segment(_ option: SpendPeriodPreference) -> some View {
            let isSelected = option == selected
            return Button {
                select(option)
            } label: {
                Text(option.title(strings))
                    .font(isSelected ? layout.type.segmentSelected : layout.type.segment)
                    .foregroundStyle(isSelected ? .primary : .secondary)
                    .lineLimit(1)
                    .minimumScaleFactor(0.8)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, layout.cg.segmentPaddingY)
                    .padding(.horizontal, 6)
                    .background {
                        if isSelected {
                            Capsule()
                                .fill(Palette.tray)
                                .shadow(color: .black.opacity(0.12), radius: 1, y: 1)
                                .matchedGeometryEffect(id: "selected", in: selection)
                        }
                    }
                    .contentShape(Capsule())
            }
            .buttonStyle(.plain)
            .accessibilityAddTraits(isSelected ? .isSelected : [])
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: selected)
        }
    }

    struct SpendBody: View {
        let model: SpendCardModel
        let formatter: DisplayFormatter
        @Environment(\.popupLayout) private var layout

        var body: some View {
            if model.isEmpty {
                Text(formatter.strings.text(.noUsageInPeriod))
                    .font(Typeface.body)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 14)
            } else {
                HStack(spacing: layout.cg.legendGap) {
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
        @Environment(\.popupLayout) private var layout

        var body: some View {
            VStack(alignment: .leading, spacing: 1) {
                HStack(spacing: 6) {
                    Circle().fill(entry.color.color).frame(width: 8, height: 8)
                    Text(entry.name).font(layout.type.body).lineLimit(1)
                    Spacer(minLength: 6)
                    Text(entry.amount)
                        .font(layout.type.bodyMedium)
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
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: entry.value)
            .contentShape(Rectangle())
            .hoverChip(horizontal: 6, vertical: 3)
            .hoverTip(id: "legend.\(entry.id)", entry.tip)
        }
    }
#endif
