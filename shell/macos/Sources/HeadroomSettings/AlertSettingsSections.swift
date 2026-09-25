#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct AlertThresholdSection: View {
        let context: SettingsContext
        let notifications: NotificationSettings

        var body: some View {
            let strings = context.strings
            Section(strings.text(AlertSettingsText.alertThreshold)) {
                SettingPicker(
                    context: context, title: strings.text(AlertSettingsText.alertWhenLessThan),
                    detail: strings.text(AlertSettingsText.alertWhenLessThanDetail),
                    value: notifications.thresholdPercent,
                    options: SettingsOptions.thresholds(current: notifications.thresholdPercent),
                    label: { "\($0)%" }, change: { .alerts(.threshold($0)) }, segmented: true)
                ForEach(SettingsOptions.alertProviders(context.model.orderedAccounts)) { provider in
                    providerRow(provider)
                }
            }
        }

        private func providerRow(_ provider: AlertProvider) -> some View {
            let strings = context.strings
            let current = notifications.providerThresholds[provider.id]
            let general = notifications.thresholdPercent
            return Picker(
                selection: context.binding(current) { .alerts(.providerThreshold(provider: provider.id, percent: $0)) }
            ) {
                ForEach(SettingsOptions.providerThresholds(current: current), id: \.self) { percent in
                    Text(SettingsOptions.thresholdLabel(percent, general: general, strings: strings)).tag(percent)
                }
            } label: {
                HStack(spacing: 8) {
                    ProviderMark(context: context, providerID: provider.id, name: provider.name, size: 18)
                    Text(provider.name)
                }
                .padding(.leading, 12)
            }
        }
    }

    struct QuietHoursSection: View {
        let context: SettingsContext
        let quietHours: QuietHours

        var body: some View {
            let strings = context.strings
            Section {
                SettingToggle(
                    context, AlertSettingsText.quietHours, AlertSettingsText.quietHoursDetail, quietHours.enabled
                ) {
                    .alerts(.quietHours(.enabled($0)))
                }
                Group {
                    timePicker(AlertSettingsText.from, quietHours.from) { .from($0) }
                    timePicker(AlertSettingsText.to, quietHours.to) { .to($0) }
                    SettingToggle(
                        context, AlertSettingsText.allowCritical, AlertSettingsText.allowCriticalDetail,
                        quietHours.allowCritical
                    ) { .alerts(.quietHours(.allowCritical($0))) }
                }
                .disabled(!quietHours.enabled)
            } header: {
                Text(strings.text(AlertSettingsText.quietHours))
            } footer: {
                Text(strings.text(AlertSettingsText.quietHoursFooter)).foregroundStyle(.secondary)
            }
        }

        private func timePicker(
            _ title: AlertSettingsText, _ time: TimeOfDay, _ field: @escaping (TimeOfDay) -> QuietHoursField
        ) -> some View {
            let calendar = Calendar.current
            let store = context.store
            let binding = Binding(
                get: { time.date(on: Date(), calendar: calendar) },
                set: { store.change(.alerts(.quietHours(field(TimeOfDay(date: $0, calendar: calendar))))) })
            return DatePicker(context.strings.text(title), selection: binding, displayedComponents: .hourAndMinute)
                .datePickerStyle(.stepperField)
        }
    }
#endif
