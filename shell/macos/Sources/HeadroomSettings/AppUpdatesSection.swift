#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct AppUpdatesSection: View {
        let updates: UpdatesModel
        let context: SettingsContext

        var body: some View {
            Toggle(
                isOn: Binding(get: { updates.status.automaticallyChecks }, set: { updates.setAutomaticallyChecks($0) })
            ) {
                TitledLabel(
                    title: strings.text(UpdateText.automaticChecks),
                    detail: strings.text(UpdateText.automaticChecksDetail))
            }
            HStack {
                Text(context.model.formatter.lastUpdateCheckText(updates.status.lastCheck, now: Date()))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
                Spacer()
                Button(strings.text(UpdateText.checkNow)) { updates.checkNow() }
                    .disabled(!updates.status.canCheck)
            }
            .onAppear { updates.refresh() }
        }

        private var strings: UIStrings { context.strings }
    }
#endif
