#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    struct SignInFlowView: View {
        let context: SettingsContext
        let provider: ProviderInfo
        let onClose: () -> Void

        @State private var session: AddAccountSession
        @State private var label = ""
        @State private var code = ""
        @State private var codeSent = false
        @Environment(\.openURL) private var openURL

        init(context: SettingsContext, provider: ProviderInfo, onClose: @escaping () -> Void) {
            self.context = context
            self.provider = provider
            self.onClose = onClose
            _session = State(initialValue: AddAccountSession(provider: provider, launcher: context.makeLauncher()))
        }

        var body: some View {
            let strings = context.strings
            ScrollView {
                VStack(spacing: 16) {
                    FlowHeader(
                        context: context, provider: provider,
                        title: strings.fill(SignInText.signInTo, ["provider": provider.displayName]),
                        description: description(strings))
                    content(strings)
                }
                .padding(24)
            }
            .onDisappear { session.cancel() }
        }

        private func description(_ strings: UIStrings) -> String {
            switch session.phase {
            case .form: strings.fill(SignInText.cliIntro, ["provider": provider.displayName])
            case .running(let progress):
                strings.text(progress.url == nil ? SignInText.takesAMoment : SignInText.openPageHint)
            case .done, .failed: ""
            }
        }

        @ViewBuilder
        private func content(_ strings: UIStrings) -> some View {
            switch session.phase {
            case .form:
                form(strings)
            case .running(let progress):
                running(progress, strings)
            case .done, .failed:
                FlowResult(strings: strings, phase: session.phase, onClose: onClose) { session.cancel() }
            }
        }

        private func form(_ strings: UIStrings) -> some View {
            VStack(spacing: 12) {
                TextField(strings.text(AddAccountText.labelOptional), text: $label)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit { start() }
                Button(strings.text(SignInText.continueAction)) { start() }.keyboardShortcut(.defaultAction)
            }
        }

        private func start() {
            codeSent = false
            session.start(label: label)
        }

        private func running(_ progress: SignInProgress, _ strings: UIStrings) -> some View {
            VStack(spacing: 12) {
                HStack(spacing: 8) {
                    ProgressView().controlSize(.small)
                    Text(strings.text(progress.url == nil ? SignInText.startingSignIn : SignInText.waitingForSignIn))
                        .font(.headline)
                }
                if let deviceCode = progress.deviceCode { DeviceCodeView(code: deviceCode, strings: strings) }
                if let url = progress.url.flatMap(URL.init(string:)) {
                    Button(strings.text(SignInText.openSignInPage)) { openURL(url) }
                        .keyboardShortcut(.defaultAction)
                    codeEntry(strings)
                }
                LogView(lines: progress.log)
                Button(strings.text(SignInText.cancel)) { session.cancel() }
            }
        }

        private func codeEntry(_ strings: UIStrings) -> some View {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    TextField(strings.text(SignInText.pasteCode), text: $code)
                        .textFieldStyle(.roundedBorder)
                        .onSubmit { sendCode() }
                    Button(strings.text(SignInText.send)) { sendCode() }
                        .disabled(code.trimmingCharacters(in: .whitespaces).isEmpty)
                }
                Text(strings.text(codeSent ? SignInText.codeSent : SignInText.pasteCodeHint))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }

        private func sendCode() {
            session.sendCode(code)
            code = ""
            codeSent = true
        }
    }

    struct DeviceCodeView: View {
        let code: String
        let strings: UIStrings

        var body: some View {
            VStack(spacing: 6) {
                Text(strings.text(SignInText.yourCode)).font(.caption).foregroundStyle(.secondary)
                HStack(spacing: 10) {
                    Text(code).font(.system(.title, design: .monospaced).weight(.semibold)).textSelection(.enabled)
                    Button(strings.text(SignInText.copy)) {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(code, forType: .string)
                    }
                }
            }
            .padding(12)
            .frame(maxWidth: .infinity)
            .background(.quaternary, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        }
    }

    struct LogView: View {
        static let height: CGFloat = 140

        let lines: [String]

        var body: some View {
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 2) {
                        ForEach(Array(lines.enumerated()), id: \.offset) { index, line in
                            Text(line).id(index)
                        }
                    }
                    .font(.system(.caption, design: .monospaced))
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(8)
                }
                .frame(height: Self.height)
                .background(.quaternary.opacity(0.5), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
                .onChange(of: lines.count) { proxy.scrollTo(lines.count - 1, anchor: .bottom) }
            }
            .opacity(lines.isEmpty ? 0 : 1)
        }
    }
#endif
