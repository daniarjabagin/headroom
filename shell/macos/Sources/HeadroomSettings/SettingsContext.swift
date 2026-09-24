#if canImport(AppKit)
    import Foundation
    import HeadroomKit
    import Observation
    import SwiftUI

    @MainActor
    public final class SettingsContext {
        public let model: AppModel
        public let store: SettingsStore
        public let logFile: URL
        public let appVersion: String?
        public let notifications: NotificationAuthorizer?
        public let updates: UpdatesModel?
        public let loginItem = LoginItem()

        let navigation = SettingsNavigation()
        private let launcher: @MainActor () -> any HelperLaunching
        private let providerImage: @MainActor (String) -> Image?

        public init(
            model: AppModel, store: SettingsStore, logFile: URL, appVersion: String?,
            notifications: NotificationAuthorizer?, updates: UpdatesModel? = nil,
            launcher: @escaping @MainActor () -> any HelperLaunching,
            providerImage: @escaping @MainActor (String) -> Image? = { _ in nil }
        ) {
            self.model = model
            self.store = store
            self.logFile = logFile
            self.appVersion = appVersion
            self.notifications = notifications
            self.updates = updates
            self.launcher = launcher
            self.providerImage = providerImage
        }

        var strings: UIStrings { model.formatter.strings }

        func makeLauncher() -> any HelperLaunching {
            launcher()
        }

        func image(forProvider id: String) -> Image? {
            providerImage(id)
        }
    }

    enum SettingsTab: Hashable, CaseIterable {
        case general, accounts, notifications, service
    }

    struct AddAccountRequest: Identifiable, Hashable {
        let id = UUID()
        let provider: String?
    }

    @MainActor
    @Observable
    final class SettingsNavigation {
        var tab: SettingsTab = .general
        var addAccount: AddAccountRequest?

        func open(_ route: SettingsRoute) {
            switch route {
            case .general: tab = .general
            case .accounts: tab = .accounts
            case .notifications: tab = .notifications
            case .service: tab = .service
            case .addAccount(let provider):
                tab = .accounts
                addAccount = AddAccountRequest(provider: provider)
            }
        }
    }
#endif
