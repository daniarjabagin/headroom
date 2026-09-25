#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct MenuBarSection: View {
        let context: SettingsContext
        let settings: HeadroomKit.Settings

        var body: some View {
            Section {
                if context.release06 {
                    modePicker
                    modeRows
                } else {
                    headlinePicker
                    labelPicker(PanelLabel.allCases.filter { $0 != PanelLabel.none })
                }
            } header: {
                Text(strings.text(MenuBarText.menuBar))
            } footer: {
                if context.release06 {
                    Text(strings.text(MenuBarSettingsText.dragHint)).foregroundStyle(.secondary)
                }
            }
        }

        private var strings: UIStrings { context.strings }
        private var display: DisplaySettings { settings.display }

        @ViewBuilder
        private var modeRows: some View {
            switch display.panelMode {
            case .headline:
                headlinePicker
                indicatorPicker
                labelPicker(PanelLabel.allCases)
            case .several:
                PanelLimitsRow(context: context, limits: display.panelLimits)
                indicatorPicker
                labelPicker(PanelLabel.allCases)
            case .icon:
                EmptyView()
            }
        }

        private var modePicker: some View {
            SettingPicker(
                context: context, title: strings.text(MenuBarSettingsText.menuBarShows),
                detail: strings.text(MenuBarSettingsText.menuBarShowsDetail), value: display.panelMode,
                options: PanelMode.allCases, label: { $0.title(strings) }, change: { .panel(.mode($0)) })
        }

        private var indicatorPicker: some View {
            SettingPicker(
                context: context, title: strings.text(MenuBarSettingsText.indicatorStyle),
                detail: strings.text(MenuBarSettingsText.indicatorStyleDetail), value: display.panelIndicator,
                options: PanelIndicator.allCases, label: { $0.title(strings) }, change: { .panel(.indicator($0)) },
                segmented: true)
        }

        private func labelPicker(_ labels: [PanelLabel]) -> some View {
            SettingPicker(
                context: context, title: strings.text(MenuBarText.menuBarLabel), detail: nil, value: display.panelLabel,
                options: labels, label: { $0.title(strings) }, change: SettingsChange.panelLabel, segmented: true)
        }

        private var headlinePicker: some View {
            let options = SettingsOptions.headlines(
                accounts: context.model.orderedAccounts, current: settings.headline, formatter: context.model.formatter)
            return Picker(selection: context.binding(settings.headline, SettingsChange.headline)) {
                ForEach(options) { Text($0.label).tag($0.setting) }
            } label: {
                TitledLabel(
                    title: strings.text(MenuBarText.menuBarLimit), detail: strings.text(MenuBarText.menuBarLimitDetail))
            }
        }
    }

    struct PanelLimitsRow: View {
        let context: SettingsContext
        let limits: [PanelLimit]

        var body: some View {
            let strings = context.strings
            let options = PanelLimitChoices.options(
                accounts: context.model.orderedAccounts, current: limits, formatter: context.model.formatter)
            LabeledContent {
                Menu(PanelLimitChoices.summary(limits, options: options, strings: strings)) {
                    ForEach(options) { option in
                        Toggle(option.label, isOn: binding(option.limit))
                            .disabled(!PanelLimitChoices.canAdd(option.limit, to: limits))
                    }
                }
                .fixedSize()
            } label: {
                TitledLabel(
                    title: strings.text(MenuBarSettingsText.limitsInMenuBar),
                    detail: strings.text(MenuBarSettingsText.limitsInMenuBarDetail))
            }
        }

        private func binding(_ limit: PanelLimit) -> Binding<Bool> {
            let store = context.store
            let current = limits
            return Binding(
                get: { current.contains(limit) },
                set: { _ in store.change(.panel(.limits(PanelLimitChoices.toggled(limit, in: current)))) })
        }
    }
#endif
