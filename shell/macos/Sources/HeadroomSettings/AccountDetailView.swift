#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct AccountDetailView: View {
        let context: SettingsContext
        let account: Account
        let onRemove: (Account) -> Void

        @State private var draftLabel = ""

        var body: some View {
            let strings = context.strings
            Form {
                Section {
                    header
                    Toggle(strings.text(AccountsText.showInMenuBar), isOn: visibility)
                    labelEditor
                }
                if !account.windows.isEmpty {
                    Section {
                        ForEach(account.windows) { window in windowToggle(window) }
                    } header: {
                        Text(strings.text(AccountsText.limits))
                    } footer: {
                        Text(strings.text(AccountsText.limitsFooter)).foregroundStyle(.secondary)
                    }
                }
                Section { removeRow }
            }
            .formStyle(.grouped)
            .onChange(of: account.id, initial: true) { draftLabel = account.label ?? "" }
        }

        private var strings: UIStrings { context.strings }

        private var header: some View {
            HStack(spacing: 12) {
                ProviderMark(context: context, providerID: account.provider, name: account.providerName, size: 32)
                VStack(alignment: .leading, spacing: 2) {
                    Text(account.displayName).font(.headline)
                    Text(SettingsOptions.accountSubtitle(account, strings: strings))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }

        private var visibility: Binding<Bool> {
            let model = context.model
            let accountID = account.id
            return Binding(
                get: { !account.hidden },
                set: { model.send(.setAccountHidden(accountID: accountID, hidden: !$0)) })
        }

        private var labelEditor: some View {
            HStack {
                TextField(strings.text(AccountsText.label), text: $draftLabel, prompt: Text(account.email ?? ""))
                    .onSubmit(saveLabel)
                Button(strings.text(AccountsText.saveLabel), action: saveLabel)
                    .disabled(trimmedDraft == (account.label ?? ""))
            }
        }

        private var trimmedDraft: String {
            draftLabel.trimmingCharacters(in: .whitespacesAndNewlines)
        }

        private func saveLabel() {
            guard trimmedDraft != (account.label ?? "") else { return }
            context.model.send(.setAccountLabel(accountID: account.id, label: trimmedDraft))
        }

        private func windowToggle(_ window: QuotaWindow) -> some View {
            let formatter = context.model.formatter
            let display = context.store.settings?.display
            let shown = Binding(
                get: { !(display?.isHidden(accountID: account.id, windowID: window.id) ?? window.hidden) },
                set: { setWindow(window.id, hidden: !$0) })
            let reading = formatter.percentReading(window.remainingPercent, mode: .left)
            return Toggle(isOn: shown) {
                TitledLabel(title: formatter.windowLabel(id: window.id, label: window.label), detail: reading)
            }
            .disabled(display == nil)
        }

        private func setWindow(_ windowID: String, hidden: Bool) {
            guard let display = context.store.settings?.display else { return }
            let windows = display.hiddenWindows(after: windowID, hidden: hidden, accountID: account.id)
            context.store.change(.hiddenWindows(accountID: account.id, windows: windows))
        }

        private var removeRow: some View {
            HStack {
                TitledLabel(
                    title: strings.text(RemovalText.removeFromHeadroom),
                    detail: SettingsOptions.removalDetail(account, strings: strings))
                Spacer()
                Button(strings.text(RemovalText.removeEllipsis), role: .destructive) { onRemove(account) }
            }
        }
    }
#endif
