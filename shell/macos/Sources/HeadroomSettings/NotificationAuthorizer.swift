#if canImport(AppKit)
    import Foundation
    import Observation
    import UserNotifications

    public enum NotificationPermission: Sendable, Hashable {
        case unknown, notDetermined, allowed, denied
    }

    @MainActor
    @Observable
    public final class NotificationAuthorizer {
        public private(set) var permission = NotificationPermission.unknown

        public init() {}

        public func refresh() async {
            permission = await Self.currentPermission()
        }

        @discardableResult
        public func requestIfNeeded() async -> Bool {
            await refresh()
            if permission == .notDetermined {
                await Self.requestAuthorization()
                await refresh()
            }
            return permission == .allowed
        }

        private nonisolated static func currentPermission() async -> NotificationPermission {
            let status = await UNUserNotificationCenter.current().notificationSettings().authorizationStatus
            switch status {
            case .notDetermined: return .notDetermined
            case .denied: return .denied
            case .authorized, .provisional: return .allowed
            @unknown default: return .allowed
            }
        }

        private nonisolated static func requestAuthorization() async {
            _ = try? await UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound])
        }
    }
#endif
