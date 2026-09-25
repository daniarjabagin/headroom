#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct FlowView: View {
        let context: SettingsContext
        let provider: ProviderInfo
        let method: AddAccountMethod
        let accountID: String?
        let onClose: () -> Void

        var body: some View {
            Group {
                switch method {
                case .cliLogin:
                    SignInFlowView(context: context, provider: provider, accountID: accountID, onClose: onClose)
                case .apiKey(let label, let consoleURL, let hint):
                    APIKeyFlowView(
                        context: context, provider: provider, field: APIKeyField(label, consoleURL, hint),
                        accountID: accountID, onClose: onClose)
                case .autoDetect(let reason):
                    AutoDetectFlowView(context: context, provider: provider, reason: reason)
                case .unsupported:
                    EmptyView()
                }
            }
            .navigationTitle(title)
        }

        private var title: String {
            let values = ["provider": provider.displayName]
            guard accountID != nil else { return context.strings.fill(AddAccountText.addProviderAccount, values) }
            return context.strings.fill(SignInAgainText.signInAgainTitle, values)
        }
    }

    struct FlowHeader: View {
        let context: SettingsContext
        let provider: ProviderInfo
        let title: String
        let description: String

        var body: some View {
            VStack(spacing: 10) {
                ProviderMark(context: context, providerID: provider.id, name: provider.displayName, size: 48)
                Text(title).font(.title2.weight(.semibold)).multilineTextAlignment(.center)
                Text(description)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .frame(maxWidth: .infinity)
        }
    }

    struct FlowResult: View {
        let strings: UIStrings
        let phase: AddAccountPhase
        var signingInAgain = false
        let onClose: () -> Void
        let onRetry: () -> Void

        var body: some View {
            switch phase {
            case .done:
                VStack(spacing: 12) {
                    Image(systemName: "checkmark.circle.fill").font(.largeTitle).foregroundStyle(.green)
                    Text(strings.text(signingInAgain ? SignInAgainText.signedInAgain : SignInText.accountAdded))
                        .multilineTextAlignment(.center)
                    Button(strings.text(SignInText.done)) { onClose() }.keyboardShortcut(.defaultAction)
                }
            case .failed(let failure):
                VStack(spacing: 12) {
                    Image(systemName: "exclamationmark.triangle.fill").font(.largeTitle).foregroundStyle(.orange)
                    Text(failure.text(strings)).multilineTextAlignment(.center).textSelection(.enabled)
                    Button(strings.text(SignInText.tryAgain)) { onRetry() }.keyboardShortcut(.defaultAction)
                }
            case .form, .running:
                EmptyView()
            }
        }
    }

    struct APIKeyField {
        let label: String
        let consoleURL: URL?
        let hint: String

        init(_ label: String, _ consoleURL: String, _ hint: String) {
            self.label = label
            self.consoleURL = URL(string: consoleURL).flatMap { $0.scheme == "https" ? $0 : nil }
            self.hint = hint
        }
    }

    struct AutoDetectFlowView: View {
        let context: SettingsContext
        let provider: ProviderInfo
        let reason: String

        @State private var status: DetectStatus = .idle

        enum DetectStatus: Hashable {
            case idle, looking, finished, unreachable
        }

        var body: some View {
            let strings = context.strings
            VStack(spacing: 16) {
                FlowHeader(
                    context: context, provider: provider, title: provider.displayName,
                    description: reason.isEmpty ? strings.text(AddAccountText.autoDetectFallback) : reason)
                Text(strings.text(AddAccountText.detectAgainNote)).font(.callout).multilineTextAlignment(.center)
                Button(strings.text(AddAccountText.detectAgain)) { detect() }
                    .keyboardShortcut(.defaultAction)
                    .disabled(status == .looking)
                statusText(strings).font(.caption).foregroundStyle(.secondary)
                Spacer()
            }
            .padding(24)
        }

        @ViewBuilder
        private func statusText(_ strings: UIStrings) -> some View {
            switch status {
            case .idle: EmptyView()
            case .looking: Text(strings.text(AddAccountText.lookingForAccounts))
            case .finished: Text(strings.text(AddAccountText.scanFinished))
            case .unreachable: Text(strings.text(AddAccountText.serviceUnreachable))
            }
        }

        private func detect() {
            status = .looking
            let task = context.model.send(.restoreAccounts(provider: provider.id))
            Task {
                let outcome = await task?.value
                if case .some(.success) = outcome { status = .finished } else { status = .unreachable }
            }
        }
    }
#endif
