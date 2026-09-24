#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    struct AccountList: View {
        let state: DaemonState
        let accounts: [Account]
        let formatter: DisplayFormatter

        var body: some View {
            if accounts.isEmpty {
                StatusMessage(title: formatter.strings.text(.noAccounts), detail: nil)
            } else {
                TimelineView(.periodic(from: .now, by: 30)) { context in
                    VStack(alignment: .leading, spacing: PopupMetrics.sectionGap) {
                        ForEach(accounts) { account in
                            AccountSection(
                                account: account, display: state.display, formatter: formatter,
                                now: Timestamp(date: context.date))
                        }
                    }
                }
            }
        }
    }

    struct AccountSection: View {
        let account: Account
        let display: DisplaySettings
        let formatter: DisplayFormatter
        let now: Timestamp

        var body: some View {
            VStack(alignment: .leading, spacing: 4) {
                header
                VStack(alignment: .leading, spacing: 0) {
                    statusLine
                    ForEach(account.windows.filter { !$0.hidden }) { window in
                        WindowRow(window: window, display: display, formatter: formatter, now: now)
                    }
                    ForEach(account.notices, id: \.self) { notice in
                        Text(notice.text)
                            .font(.system(size: 11))
                            .foregroundStyle(notice.tone.color)
                            .padding(.horizontal, PopupMetrics.padding)
                            .padding(.vertical, 6)
                    }
                }
                .cardSurface()
            }
        }

        private var header: some View {
            HStack(alignment: .firstTextBaseline, spacing: 6) {
                Text(title).font(.system(size: 14, weight: .semibold))
                if let plan = account.plan {
                    Text(plan).font(.system(size: 11)).foregroundStyle(.secondary)
                }
                Spacer(minLength: 0)
                if account.status == .refreshing {
                    ProgressView().controlSize(.mini)
                }
            }
        }

        private var title: String {
            guard let name = account.label ?? account.email else { return account.providerName }
            return "\(account.providerName): \(name)"
        }

        @ViewBuilder
        private var statusLine: some View {
            if let message = statusMessage {
                Text(message)
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, PopupMetrics.padding)
                    .padding(.vertical, 10)
            }
        }

        private var statusMessage: String? {
            switch account.status {
            case .signedOut: account.error.map(\.message) ?? formatter.strings.text(.signedOut)
            case .noSubscription: account.error.map(\.message) ?? formatter.strings.text(.noSubscription)
            case .error: account.error?.message
            default: account.windows.isEmpty ? formatter.strings.text(.noData) : nil
            }
        }
    }
#endif
