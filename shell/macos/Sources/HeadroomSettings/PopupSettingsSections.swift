#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct SpendSection: View {
        let context: SettingsContext
        let display: DisplaySettings

        var body: some View {
            let strings = context.strings
            Section(strings.text(SpendSettingsText.spend)) {
                SettingToggle(
                    context, SpendSettingsText.showSpend, SpendSettingsText.showSpendDetail, display.showSpend
                ) {
                    .section(.showSpend, $0)
                }
                SettingPicker(
                    context: context, title: strings.text(SpendSettingsText.defaultPeriod),
                    detail: strings.text(SpendSettingsText.defaultPeriodDetail), value: display.spendPeriod,
                    options: SpendPeriodPreference.allCases, label: { $0.title(strings) },
                    change: { .spend(.period($0)) })
                SettingPicker(
                    context: context, title: strings.text(SpendSettingsText.units),
                    detail: display.spendUnit.detail(strings),
                    value: display.spendUnit, options: SpendUnit.allCases, label: { $0.title(strings) },
                    change: { .spend(.unit($0)) })
                SettingPicker(
                    context: context, title: strings.text(SpendSettingsText.breakdown),
                    detail: strings.text(SpendSettingsText.breakdownDetail), value: display.spendBreakdown,
                    options: SpendBreakdown.allCases, label: { $0.title(strings) }, change: { .spend(.breakdown($0)) },
                    segmented: true)
                SettingToggle(
                    context, SpendSettingsText.showBreakdown, SpendSettingsText.showBreakdownDetail,
                    display.showBreakdown
                ) { .spend(.showBreakdown($0)) }
            }
        }
    }

    struct PopupCardsSection: View {
        let context: SettingsContext
        let display: DisplaySettings

        var body: some View {
            let strings = context.strings
            let accounts = context.model.visibleAccounts
            Section {
                SettingToggle(
                    context, CardsSettingsText.collapseUnstarred, CardsSettingsText.collapseUnstarredDetail,
                    display.collapseUnstarred
                ) { .collapseUnstarred($0) }
                ForEach(accounts) { account in
                    StarredAccountRow(context: context, account: account, among: accounts, display: display)
                }
            } header: {
                Text(strings.text(CardsSettingsText.popupCards))
            } footer: {
                Text(strings.text(CardsSettingsText.starFooter)).foregroundStyle(.secondary)
            }
        }
    }

    struct StarredAccountRow: View {
        let context: SettingsContext
        let account: Account
        let among: [Account]
        let display: DisplaySettings

        var body: some View {
            let strings = context.strings
            let starred = display.isStarred(accountID: account.id)
            HStack(spacing: 10) {
                ProviderMark(context: context, providerID: account.provider, name: account.providerName, size: 20)
                Text(SettingsOptions.accountTitle(account, among: among)).lineLimit(1)
                Spacer()
                Text(strings.text(starred ? CardsSettingsText.alwaysOpen : CardsSettingsText.onDemand))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Button {
                    context.store.change(
                        .starredAccounts(display.starredAccounts(after: account.id, starred: !starred)))
                } label: {
                    Image(systemName: starred ? "star.fill" : "star")
                        .foregroundStyle(starred ? Color.accentColor : Color.secondary)
                }
                .buttonStyle(.borderless)
                .accessibilityLabel(strings.text(starred ? CardsSettingsText.alwaysOpen : CardsSettingsText.onDemand))
            }
        }
    }

    struct DataRefreshSection: View {
        let context: SettingsContext
        let settings: HeadroomKit.Settings

        var body: some View {
            let strings = context.strings
            Section(strings.text(RefreshSettingsText.dataRefresh)) {
                SettingPicker(
                    context: context, title: strings.text(MenuBarText.refreshInterval),
                    detail: strings.text(MenuBarText.refreshIntervalDetail), value: settings.refreshIntervalSecs,
                    options: SettingsOptions.refreshIntervals(current: settings.refreshIntervalSecs),
                    label: { strings.refreshInterval(seconds: $0) }, change: SettingsChange.refreshInterval)
                if context.release06 {
                    SettingToggle(
                        context, RefreshSettingsText.adaptive, RefreshSettingsText.adaptiveDetail,
                        settings.adaptiveRefresh
                    ) { .adaptiveRefresh($0) }
                }
            }
        }
    }
#endif
