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
                    if context.release06 { SpendSection(context: context, display: settings.display) }
                    sections(settings.display)
                    if context.release06 { PopupCardsSection(context: context, display: settings.display) }
                    DataRefreshSection(context: context, settings: settings)
                    if context.release06 {
                        PrivacySection(context: context, settings: settings)
                        KeyboardSection(context: context, accelerator: settings.shortcuts.open)
                    }
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
                Picker(selection: context.binding(settings.display.theme, SettingsChange.theme)) {
                    Text(strings.text(AppearanceText.system)).tag(ThemePreference.system)
                    Text(strings.text(AppearanceText.light)).tag(ThemePreference.light)
                    Text(strings.text(AppearanceText.dark)).tag(ThemePreference.dark)
                } label: {
                    Text(strings.text(AppearanceText.theme))
                }
                .pickerStyle(.segmented)
                Picker(selection: context.binding(settings.display.language, SettingsChange.language)) {
                    Text(strings.text(AppearanceText.system)).tag(LanguagePreference.system)
                    Text(verbatim: "English").tag(LanguagePreference.en)
                    Text(verbatim: "Русский").tag(LanguagePreference.ru)
                } label: {
                    Text(strings.text(AppearanceText.language))
                }
                .pickerStyle(.segmented)
                if context.release06 { layout(settings.display) }
                SettingToggle(
                    context, AppearanceText.translucent, AppearanceText.translucentDetail, settings.display.translucent
                ) { .translucent($0) }
                SettingToggle(
                    context, AppearanceText.reducedMotion, AppearanceText.reducedMotionDetail, settings.reducedMotion
                ) { .reducedMotion($0) }
            }
        }

        @ViewBuilder
        private func layout(_ display: DisplaySettings) -> some View {
            SettingPicker(
                context: context, title: strings.text(LayoutSettingsText.timeFormat),
                detail: strings.text(LayoutSettingsText.timeFormatDetail), value: display.timeFormat,
                options: TimeFormat.allCases, label: { $0.title(strings) }, change: SettingsChange.timeFormat,
                segmented: true)
            SettingPicker(
                context: context, title: strings.text(LayoutSettingsText.density),
                detail: strings.text(LayoutSettingsText.densityDetail), value: display.density,
                options: Density.allCases, label: { $0.title(strings) }, change: SettingsChange.density,
                segmented: true)
        }

        private func popup(_ display: DisplaySettings) -> some View {
            Section(strings.text(AppearanceText.popup)) {
                Picker(selection: context.binding(display.valueMode, SettingsChange.valueMode)) {
                    Text(strings.text(AppearanceText.left)).tag(ValueMode.left)
                    Text(strings.text(AppearanceText.used)).tag(ValueMode.used)
                } label: {
                    TitledLabel(
                        title: strings.text(AppearanceText.valueMode),
                        detail: strings.text(AppearanceText.valueModeDetail))
                }
                .pickerStyle(.segmented)
                Picker(selection: context.binding(display.resetFormat, SettingsChange.resetFormat)) {
                    Text(strings.text(AppearanceText.countdown)).tag(ResetFormat.countdown)
                    Text(strings.text(AppearanceText.exactTime)).tag(ResetFormat.exact)
                } label: {
                    TitledLabel(
                        title: strings.text(AppearanceText.resetFormat),
                        detail: strings.text(AppearanceText.resetFormatDetail))
                }
                .pickerStyle(.segmented)
                SettingToggle(
                    context, CombinedText.combineAccounts, CombinedText.combineAccountsDetail, display.combineAccounts
                ) { .combineAccounts($0) }
            }
        }

        private func sections(_ display: DisplaySettings) -> some View {
            Section(strings.text(MenuBarText.sections)) {
                ForEach(Self.sections(release06: context.release06), id: \.self) { section in
                    let texts = Self.texts(section)
                    SettingToggle(context, texts.title, texts.detail, display.isShown(section)) {
                        .section(section, $0)
                    }
                }
            }
        }

        private static func sections(release06: Bool) -> [DisplaySection] {
            release06 ? DisplaySection.allCases.filter { $0 != .showSpend } : DisplaySection.allCases
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
