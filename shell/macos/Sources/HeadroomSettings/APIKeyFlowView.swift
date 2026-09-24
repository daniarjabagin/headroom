#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct APIKeyFlowView: View {
        let context: SettingsContext
        let provider: ProviderInfo
        let field: APIKeyField
        let onClose: () -> Void

        @State private var session: AddAccountSession
        @State private var key = ""
        @State private var label = ""
        @Environment(\.openURL) private var openURL

        init(context: SettingsContext, provider: ProviderInfo, field: APIKeyField, onClose: @escaping () -> Void) {
            self.context = context
            self.provider = provider
            self.field = field
            self.onClose = onClose
            _session = State(initialValue: AddAccountSession(provider: provider, launcher: context.makeLauncher()))
        }

        var body: some View {
            let strings = context.strings
            VStack(spacing: 16) {
                FlowHeader(
                    context: context, provider: provider,
                    title: strings.fill(SignInText.connectProvider, ["provider": provider.displayName]),
                    description: session.phase == .form
                        ? strings.fill(SignInText.apiKeyIntro, ["provider": provider.displayName]) : "")
                content(strings)
                Spacer()
            }
            .padding(24)
            .onDisappear { session.cancel() }
        }

        @ViewBuilder
        private func content(_ strings: UIStrings) -> some View {
            switch session.phase {
            case .form:
                form(strings)
            case .running:
                VStack(spacing: 12) {
                    HStack(spacing: 8) {
                        ProgressView().controlSize(.small)
                        Text(strings.text(SignInText.checkingKey)).font(.headline)
                    }
                    Button(strings.text(SignInText.cancel)) { session.cancel() }
                }
            case .done, .failed:
                FlowResult(strings: strings, phase: session.phase, onClose: onClose) { session.cancel() }
            }
        }

        private func form(_ strings: UIStrings) -> some View {
            VStack(alignment: .leading, spacing: 10) {
                SecureField(field.label.isEmpty ? strings.text(AddAccountText.apiKey) : field.label, text: $key)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit { add() }
                TextField(strings.text(AddAccountText.labelOptional), text: $label)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit { add() }
                if !field.hint.isEmpty {
                    Text(field.hint).font(.caption).foregroundStyle(.secondary)
                }
                HStack {
                    Spacer()
                    if let consoleURL = field.consoleURL {
                        Button(strings.text(SignInText.getKey)) { openURL(consoleURL) }
                    }
                    Button(strings.text(SignInText.add)) { add() }
                        .keyboardShortcut(.defaultAction)
                        .disabled(trimmedKey.isEmpty)
                }
            }
        }

        private var trimmedKey: String {
            key.trimmingCharacters(in: .whitespacesAndNewlines)
        }

        private func add() {
            let submitted = trimmedKey
            guard !submitted.isEmpty else { return }
            key = ""
            session.start(label: label, apiKey: submitted)
        }
    }
#endif
