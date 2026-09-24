#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    struct ServiceSettingsView: View {
        let context: SettingsContext

        var body: some View {
            Form {
                Section { about }
                Section(strings.text(ServiceText.serviceStatus)) { status }
                Section(strings.text(ServiceText.logs)) { logs }
                Section(strings.text(ServiceText.startup)) { startup }
            }
            .formStyle(.grouped)
        }

        private var strings: UIStrings { context.strings }

        private var about: some View {
            HStack(spacing: 12) {
                Image(nsImage: NSApp.applicationIconImage).resizable().frame(width: 48, height: 48)
                VStack(alignment: .leading, spacing: 2) {
                    Text(verbatim: "Headroom").font(.title3.weight(.semibold))
                    if let version = context.appVersion {
                        Text(strings.fill(ServiceText.version, ["version": version]))
                            .foregroundStyle(.secondary)
                            .monospacedDigit()
                    }
                }
            }
        }

        @ViewBuilder
        private var status: some View {
            LabeledContent(strings.text(ServiceText.serviceStatus), value: statusText)
            if let issue = context.model.serviceIssue {
                Text(issue).font(.caption).foregroundStyle(.secondary).textSelection(.enabled)
            }
        }

        private var statusText: String {
            switch context.model.phase {
            case .connected: strings.text(ServiceText.running)
            case .starting, .connecting: strings.text(ServiceText.connectingToHeadroom)
            case .disconnected: strings.text(ServiceText.notRunning)
            case .incompatible(let text): strings.text(text)
            }
        }

        @ViewBuilder
        private var logs: some View {
            LabeledContent(strings.text(ServiceText.daemonLog)) {
                Text(context.logFile.path).font(.caption).textSelection(.enabled).lineLimit(2)
            }
            HStack {
                Button(strings.text(ServiceText.openLog)) { NSWorkspace.shared.open(context.logFile) }
                Button(strings.text(ServiceText.showInFinder)) {
                    NSWorkspace.shared.activateFileViewerSelecting([context.logFile])
                }
            }
        }

        @ViewBuilder
        private var startup: some View {
            let loginItem = context.loginItem
            Toggle(isOn: Binding(get: { loginItem.isEnabled }, set: { loginItem.setEnabled($0) })) {
                TitledLabel(
                    title: strings.text(ServiceText.launchAtLogin),
                    detail: strings.text(ServiceText.launchAtLoginDetail))
            }
            if loginItem.needsApproval {
                HStack {
                    Text(strings.text(ServiceText.loginItemNeedsApproval)).font(.caption)
                    Spacer()
                    Button(strings.text(ServiceText.openLoginItems)) { loginItem.openSystemSettings() }
                }
            }
            if let failure = loginItem.failure {
                Text(failure).font(.caption).foregroundStyle(.red)
            }
        }
    }
#endif
