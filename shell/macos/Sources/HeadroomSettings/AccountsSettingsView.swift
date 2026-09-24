#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct AccountsSettingsView: View {
        static let listWidth: CGFloat = 250

        let context: SettingsContext

        @State private var selection: Account.ID?
        @State private var pendingRemoval: Account?
        @State private var removal = RemovalStatus.idle

        var body: some View {
            let strings = context.strings
            HStack(spacing: 0) {
                VStack(alignment: .leading, spacing: 8) {
                    accountList
                    Button(strings.text(AccountsText.addAccountEllipsis)) {
                        context.navigation.addAccount = AddAccountRequest(provider: nil)
                    }
                    Text(strings.text(AccountsText.accountsFooter))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                    RemovalStatusView(status: removal, strings: strings)
                }
                .padding(12)
                .frame(width: Self.listWidth)
                Divider()
                detail
            }
            .alert(
                strings.fill(RemovalText.removeTitle, ["name": pendingRemoval?.displayName ?? ""]),
                isPresented: Binding(get: { pendingRemoval != nil }, set: { if !$0 { pendingRemoval = nil } }),
                presenting: pendingRemoval
            ) { account in
                Button(strings.text(RemovalText.remove), role: .destructive) { remove(account) }
                Button(strings.text(RemovalText.cancel), role: .cancel) {}
            } message: { account in
                Text(SettingsOptions.removalBody(account, strings: strings))
            }
        }

        private var accountList: some View {
            let accounts = context.model.orderedAccounts
            return List(selection: $selection) {
                ForEach(accounts) { account in
                    AccountListRow(context: context, account: account, among: accounts).tag(account.id)
                }
                .onMove { offsets, destination in
                    let ids = accounts.map(\.id)
                    context.model.setAccountOrder(
                        SettingsOptions.reordered(ids, moving: Array(offsets), to: destination))
                }
            }
            .listStyle(.inset(alternatesRowBackgrounds: false))
            .overlay {
                if accounts.isEmpty {
                    ContentUnavailableView(
                        context.strings.text(AccountsText.noAccounts), systemImage: "person.crop.circle.badge.plus",
                        description: Text(context.strings.text(AccountsText.noAccountsDetail)))
                }
            }
        }

        @ViewBuilder
        private var detail: some View {
            if let account = context.model.orderedAccounts.first(where: { $0.id == selection }) {
                AccountDetailView(context: context, account: account) { pendingRemoval = $0 }
            } else {
                Text(context.strings.text(AccountsText.selectAccount))
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }

        private func remove(_ account: Account) {
            removal = .running
            let launcher = context.makeLauncher()
            Task {
                let outcome = await AccountRemoval.remove(accountID: account.id, launcher: launcher)
                switch outcome {
                case .success: removal = .removed
                case .failure(let failure): removal = .failed(failure)
                }
            }
        }
    }

    enum RemovalStatus: Hashable {
        case idle, running, removed
        case failed(HelperFailure)
    }

    struct RemovalStatusView: View {
        let status: RemovalStatus
        let strings: UIStrings

        var body: some View {
            switch status {
            case .idle: EmptyView()
            case .running:
                HStack(spacing: 6) {
                    ProgressView().controlSize(.small)
                    Text(strings.text(RemovalText.removing))
                }
                .font(.caption)
            case .removed:
                Label(strings.text(RemovalText.accountRemoved), systemImage: "checkmark.circle").font(.caption)
            case .failed(let failure):
                Label(failure.text(strings), systemImage: "exclamationmark.triangle")
                    .font(.caption)
                    .foregroundStyle(.red)
            }
        }
    }

    struct AccountListRow: View {
        let context: SettingsContext
        let account: Account
        let among: [Account]

        var body: some View {
            HStack(spacing: 8) {
                ProviderMark(context: context, providerID: account.provider, name: account.providerName, size: 24)
                VStack(alignment: .leading, spacing: 2) {
                    Text(SettingsOptions.accountTitle(account, among: among)).lineLimit(1)
                    Text(SettingsOptions.accountSubtitle(account, strings: context.strings))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
                Spacer(minLength: 0)
                if account.hidden {
                    Image(systemName: "eye.slash").foregroundStyle(.secondary)
                }
            }
            .opacity(account.hidden ? 0.6 : 1)
            .padding(.vertical, 2)
        }
    }
#endif
