#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct GeneralSettingsView: View {
        let context: SettingsContext

        var body: some View {
            if let settings = context.store.settings {
                Form {
                    appearance(settings)
                    popup(settings.display)
                    MenuBarSection(context: context, settings: settings)
                    sections(settings.display)
                    updates(settings)
                    StoreErrorSection(context: context)
                }
                .formStyle(.grouped)
            } else {
                SettingsPlaceholder(context: context)
            }
        }

        private var strings: UIStrings { context.strings }

        private func appearance(_ settings: HeadroomKit.Settings) -> some View {
            Section(strings.text(AppearanceText.appearance)) {
                Picker(selection: bind(settings.display.theme, SettingsChange.theme)) {
                    Text(strings.text(AppearanceText.system)).tag(ThemePreference.system)
                    Text(strings.text(AppearanceText.light)).tag(ThemePreference.light)
                    Text(strings.text(AppearanceText.dark)).tag(ThemePreference.dark)
                } label: {
                    Text(strings.text(AppearanceText.theme))
                }
                .pickerStyle(.segmented)
                Picker(selection: bind(settings.display.language, SettingsChange.language)) {
                    Text(strings.text(AppearanceText.system)).tag(LanguagePreference.system)
                    Text(verbatim: "English").tag(LanguagePreference.en)
                    Text(verbatim: "Русский").tag(LanguagePreference.ru)
                } label: {
                    Text(strings.text(AppearanceText.language))
                }
                .pickerStyle(.segmented)
                toggle(AppearanceText.translucent, AppearanceText.translucentDetail, settings.display.translucent) {
                    .translucent($0)
                }
                toggle(AppearanceText.reducedMotion, AppearanceText.reducedMotionDetail, settings.reducedMotion) {
                    .reducedMotion($0)
                }
            }
        }

        private func popup(_ display: DisplaySettings) -> some View {
            Section(strings.text(AppearanceText.popup)) {
                Picker(selection: bind(display.valueMode, SettingsChange.valueMode)) {
                    Text(strings.text(AppearanceText.left)).tag(ValueMode.left)
                    Text(strings.text(AppearanceText.used)).tag(ValueMode.used)
                } label: {
                    TitledLabel(
                        title: strings.text(AppearanceText.valueMode),
                        detail: strings.text(AppearanceText.valueModeDetail))
                }
                .pickerStyle(.segmented)
                Picker(selection: bind(display.resetFormat, SettingsChange.resetFormat)) {
                    Text(strings.text(AppearanceText.countdown)).tag(ResetFormat.countdown)
                    Text(strings.text(AppearanceText.exactTime)).tag(ResetFormat.exact)
                } label: {
                    TitledLabel(
                        title: strings.text(AppearanceText.resetFormat),
                        detail: strings.text(AppearanceText.resetFormatDetail))
                }
                .pickerStyle(.segmented)
            }
        }

        private func sections(_ display: DisplaySettings) -> some View {
            Section(strings.text(MenuBarText.sections)) {
                ForEach(DisplaySection.allCases, id: \.self) { section in
                    let texts = Self.texts(section)
                    toggle(texts.title, texts.detail, display.isShown(section)) { .section(section, $0) }
                }
            }
        }

        private func updates(_ settings: HeadroomKit.Settings) -> some View {
            Section(strings.text(MenuBarText.updates)) {
                Picker(selection: bind(settings.refreshIntervalSecs, SettingsChange.refreshInterval)) {
                    ForEach(SettingsOptions.refreshIntervals(current: settings.refreshIntervalSecs), id: \.self) {
                        Text(strings.refreshInterval(seconds: $0)).tag($0)
                    }
                } label: {
                    TitledLabel(
                        title: strings.text(MenuBarText.refreshInterval),
                        detail: strings.text(MenuBarText.refreshIntervalDetail))
                }
            }
        }

        private func toggle(
            _ title: some LocalizedText, _ detail: some LocalizedText, _ value: Bool,
            _ change: @escaping (Bool) -> SettingsChange
        ) -> some View {
            Toggle(isOn: bind(value, change)) {
                TitledLabel(title: strings.text(title), detail: strings.text(detail))
            }
        }

        private func bind<Value>(_ value: Value, _ change: @escaping (Value) -> SettingsChange) -> Binding<Value> {
            let store = context.store
            return Binding(get: { value }, set: { store.change(change($0)) })
        }

        private static func texts(_ section: DisplaySection) -> (title: MenuBarText, detail: MenuBarText) {
            switch section {
            case .showSpend: (.totalSpend, .totalSpendDetail)
            case .showAccountSpend: (.accountSpend, .accountSpendDetail)
            case .showTrend: (.usageTrend, .usageTrendDetail)
            case .showForecast: (.paceForecast, .paceForecastDetail)
            }
        }
    }

    struct MenuBarSection: View {
        let context: SettingsContext
        let settings: HeadroomKit.Settings

        var body: some View {
            let strings = context.strings
            let store = context.store
            let options = SettingsOptions.headlines(
                accounts: context.model.orderedAccounts, current: settings.headline, formatter: context.model.formatter)
            Section(strings.text(MenuBarText.menuBar)) {
                Picker(
                    selection: Binding(get: { settings.headline }, set: { store.change(.headline($0)) })
                ) {
                    ForEach(options) { Text($0.label).tag($0.setting) }
                } label: {
                    TitledLabel(
                        title: strings.text(MenuBarText.menuBarLimit),
                        detail: strings.text(MenuBarText.menuBarLimitDetail))
                }
                Picker(
                    selection: Binding(get: { settings.display.panelLabel }, set: { store.change(.panelLabel($0)) })
                ) {
                    Text(strings.text(MenuBarText.percent)).tag(PanelLabel.percent)
                    Text(strings.text(MenuBarText.providerAndLimit)).tag(PanelLabel.window)
                } label: {
                    Text(strings.text(MenuBarText.menuBarLabel))
                }
                .pickerStyle(.segmented)
            }
        }
    }

    struct StoreErrorSection: View {
        let context: SettingsContext

        var body: some View {
            if let error = context.store.lastError {
                Section {
                    Label(SettingsStore.describe(error), systemImage: "exclamationmark.triangle")
                        .foregroundStyle(.secondary)
                }
            }
        }
    }
#endif
