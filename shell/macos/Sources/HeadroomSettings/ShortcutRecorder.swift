#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    struct PrivacySection: View {
        let context: SettingsContext
        let settings: HeadroomKit.Settings

        var body: some View {
            Section(context.strings.text(PrivacySettingsText.privacy)) {
                SettingToggle(
                    context, PrivacySettingsText.hideOnShare, PrivacySettingsText.hideOnShareDetail,
                    settings.display.hideOnScreenShare
                ) { .hideOnScreenShare($0) }
                SettingToggle(
                    context, PrivacySettingsText.statusPages, PrivacySettingsText.statusPagesDetail,
                    settings.statusPages.enabled
                ) { .statusPages($0) }
            }
        }
    }

    struct KeyboardSection: View {
        let context: SettingsContext
        let accelerator: String

        var body: some View {
            let strings = context.strings
            Section(strings.text(KeyboardSettingsText.keyboard)) {
                LabeledContent {
                    ShortcutRecorder(context: context, accelerator: accelerator)
                } label: {
                    TitledLabel(
                        title: strings.text(KeyboardSettingsText.openHeadroom),
                        detail: strings.text(KeyboardSettingsText.openHeadroomDetail))
                }
            }
        }
    }

    @MainActor
    final class KeyDownMonitor {
        private var token: Any?

        func start(_ handler: @escaping @MainActor (ShortcutCapture) -> Void) {
            stop()
            token = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
                let capture = Self.capture(event)
                MainActor.assumeIsolated { handler(capture) }
                return nil
            }
        }

        nonisolated static func capture(_ event: NSEvent) -> ShortcutCapture {
            ShortcutEncoding.capture(
                modifiers: modifiers(event.modifierFlags), keyCode: event.keyCode,
                characters: event.charactersIgnoringModifiers)
        }

        nonisolated static func modifiers(_ flags: NSEvent.ModifierFlags) -> ShortcutModifiers {
            let pairs: [(NSEvent.ModifierFlags, ShortcutModifiers)] = [
                (.control, .control), (.option, .option), (.shift, .shift), (.command, .command),
            ]
            return pairs.reduce(into: ShortcutModifiers()) { result, pair in
                if flags.contains(pair.0) { result.insert(pair.1) }
            }
        }

        func stop() {
            guard let token else { return }
            NSEvent.removeMonitor(token)
            self.token = nil
        }
    }

    struct ShortcutRecorder: View {
        static let width: CGFloat = 150

        let context: SettingsContext
        let accelerator: String

        @State private var recording = false
        @State private var problem: KeyboardSettingsText?
        @State private var monitor = KeyDownMonitor()

        var body: some View {
            VStack(alignment: .trailing, spacing: 4) {
                HStack(spacing: 4) {
                    Button(action: toggleRecording) {
                        Text(title).monospacedDigit().frame(width: Self.width)
                    }
                    if !accelerator.isEmpty, !recording {
                        Button(action: clear) { Image(systemName: "xmark.circle.fill") }
                            .buttonStyle(.borderless)
                            .foregroundStyle(.secondary)
                            .accessibilityLabel(strings.text(KeyboardSettingsText.clearShortcut))
                    }
                }
                if recording, let problem {
                    Text(strings.text(problem)).font(.caption).foregroundStyle(.secondary)
                }
            }
            .onDisappear(perform: stopRecording)
        }

        private var strings: UIStrings { context.strings }

        private var title: String {
            if recording { return strings.text(KeyboardSettingsText.pressShortcut) }
            guard !accelerator.isEmpty else { return strings.text(KeyboardSettingsText.recordShortcut) }
            return ShortcutEncoding.symbols(accelerator) ?? accelerator
        }

        private func toggleRecording() {
            guard !recording else { return stopRecording() }
            recording = true
            problem = nil
            monitor.start { handle($0) }
        }

        private func handle(_ capture: ShortcutCapture) {
            switch capture {
            case .accelerator(let value):
                context.store.change(.shortcut(value))
                stopRecording()
            case .clear: clear()
            case .cancel: stopRecording()
            case .needsModifier: problem = .needsModifier
            case .unsupported: problem = .unsupportedKey
            }
        }

        private func clear() {
            context.store.change(.shortcut(""))
            stopRecording()
        }

        private func stopRecording() {
            monitor.stop()
            recording = false
            problem = nil
        }

    }
#endif
