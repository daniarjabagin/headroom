#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct NotificationsSettingsView: View {
        static let systemSettingsURL = URL(
            string: "x-apple.systempreferences:com.apple.Notifications-Settings.extension"
                + "?id=io.github.daniarjabagin.headroom")

        let context: SettingsContext

        @Environment(\.openURL) private var openURL

        var body: some View {
            if let settings = context.store.settings {
                Form {
                    milestones(settings.notifications)
                    permission
                    StoreErrorSection(context: context)
                }
                .formStyle(.grouped)
                .task { await context.notifications?.refresh() }
            } else {
                SettingsPlaceholder(context: context)
            }
        }

        private var strings: UIStrings { context.strings }

        private func milestones(_ notifications: NotificationSettings) -> some View {
            Section {
                ForEach(Milestone.allCases, id: \.self) { milestone in
                    let texts = Self.texts(milestone)
                    Toggle(isOn: binding(milestone, notifications.isEnabled(milestone))) {
                        TitledLabel(title: strings.text(texts.title), detail: strings.text(texts.detail))
                    }
                }
            } header: {
                Text(strings.text(NotificationText.notifyMeWhen))
            } footer: {
                Text(strings.text(NotificationText.notificationsFooter)).foregroundStyle(.secondary)
            }
        }

        @ViewBuilder
        private var permission: some View {
            if context.notifications?.permission == .denied {
                Section {
                    Label(strings.text(NotificationText.notificationsDenied), systemImage: "bell.slash")
                    if let url = Self.systemSettingsURL {
                        Button(strings.text(NotificationText.openSystemSettings)) { openURL(url) }
                    }
                }
            }
        }

        private func binding(_ milestone: Milestone, _ value: Bool) -> Binding<Bool> {
            let store = context.store
            let notifications = context.notifications
            return Binding(
                get: { value },
                set: { enabled in
                    store.change(.notification(milestone, enabled))
                    guard enabled, let notifications else { return }
                    Task { await notifications.requestIfNeeded() }
                })
        }

        private static func texts(_ milestone: Milestone) -> (title: NotificationText, detail: NotificationText) {
            switch milestone {
            case .almostOut: (.almostOut, .almostOutDetail)
            case .cuttingItClose: (.cuttingItClose, .cuttingItCloseDetail)
            case .willRunOut: (.willRunOut, .willRunOutDetail)
            case .reset: (.limitReset, .limitResetDetail)
            }
        }
    }
#endif
