#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    struct AdvancedSettingsView: View {
        static let toastDuration: Duration = .seconds(2)

        let context: SettingsContext

        @State private var confirmingReset = false
        @State private var toast: String?
        @State private var toastTask: Task<Void, Never>?

        var body: some View {
            Form {
                Section(strings.text(AdvancedText.service)) {
                    LabeledContent(strings.text(AdvancedText.headroomService), value: serviceSummary)
                }
                if context.release06, let settings = context.store.settings {
                    logging(settings.logging.level)
                    troubleshooting
                    Section {
                        Button(strings.text(AdvancedText.resetAll), role: .destructive) { confirmingReset = true }
                            .frame(maxWidth: .infinity)
                    }
                }
                StoreErrorSection(context: context)
            }
            .formStyle(.grouped)
            .overlay(alignment: .bottom) { toastView }
            .alert(strings.text(AdvancedText.resetTitle), isPresented: $confirmingReset) {
                Button(strings.text(AdvancedText.reset), role: .destructive) { context.store.resetAll() }
                Button(strings.text(AdvancedText.cancel), role: .cancel) {}
            } message: {
                Text(strings.text(AdvancedText.resetBody))
            }
            .onDisappear { toastTask?.cancel() }
        }

        private var strings: UIStrings { context.strings }

        private var serviceSummary: String {
            let status = ServiceSettingsView.statusText(context.model.phase, strings: strings)
            guard let version = context.appVersion else { return status }
            return "\(status) · \(strings.fill(ServiceText.version, ["version": version]))"
        }

        private var serviceLog: URL {
            LogFile.serviceLog(home: FileManager.default.homeDirectoryForCurrentUser)
        }

        private func logging(_ level: LogLevel) -> some View {
            Section(strings.text(AdvancedText.logging)) {
                SettingPicker(
                    context: context, title: strings.text(AdvancedText.logLevel),
                    detail: strings.text(AdvancedText.logLevelDetail), value: level, options: LogLevel.allCases,
                    label: { $0.title(strings) }, change: SettingsChange.logLevel)
                logFileRow
            }
        }

        private var logFileRow: some View {
            let url = serviceLog
            let path = LogFile.displayPath(url, home: FileManager.default.homeDirectoryForCurrentUser)
            return LabeledContent {
                HStack {
                    Button(strings.text(AdvancedText.copyPath)) {
                        copy(url.path)
                        show(strings.text(AdvancedText.copied))
                    }
                    Button(strings.text(ServiceText.showInFinder)) { reveal(url) }
                }
            } label: {
                TitledLabel(title: strings.text(AdvancedOptionText.logFile), detail: path)
            }
        }

        private var troubleshooting: some View {
            Section {
                LabeledContent {
                    Button(strings.text(AdvancedText.copy)) { copyDiagnostics() }
                } label: {
                    TitledLabel(
                        title: strings.text(AdvancedText.copyDiagnostics),
                        detail: strings.text(AdvancedText.copyDiagnosticsDetail))
                }
            } header: {
                Text(strings.text(AdvancedText.troubleshooting))
            } footer: {
                Text(strings.text(AdvancedText.troubleshootingDetail)).foregroundStyle(.secondary)
            }
        }

        @ViewBuilder
        private var toastView: some View {
            if let toast {
                Text(toast)
                    .font(.callout.weight(.medium))
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(.regularMaterial, in: Capsule())
                    .padding(.bottom, 14)
                    .transition(.opacity)
            }
        }

        private func copyDiagnostics() {
            let store = context.store
            let copied = strings.text(AdvancedText.diagnosticsCopied)
            Task {
                do throws(DaemonError) {
                    let report = try await store.diagnostics()
                    copy(report.text)
                    show(copied)
                } catch {
                    show(SettingsStore.describe(error))
                }
            }
        }

        private func show(_ message: String) {
            toastTask?.cancel()
            let animation = toastAnimation
            withAnimation(animation) { toast = message }
            toastTask = Task {
                try? await Task.sleep(for: Self.toastDuration)
                guard !Task.isCancelled else { return }
                withAnimation(animation) { toast = nil }
            }
        }

        private var toastAnimation: Animation? {
            let reduced =
                context.store.settings?.reducedMotion ?? false
                || NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
            return reduced ? nil : .easeOut(duration: 0.2)
        }

        private func copy(_ text: String) {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
        }

        private func reveal(_ url: URL) {
            guard FileManager.default.fileExists(atPath: url.path) else {
                NSWorkspace.shared.open(url.deletingLastPathComponent())
                return
            }
            NSWorkspace.shared.activateFileViewerSelecting([url])
        }
    }
#endif
