#if canImport(AppKit)
    import Foundation
    import HeadroomKit
    import HeadroomSettings
    import UserNotifications

    @MainActor
    final class AlertPoster: NSObject, UNUserNotificationCenterDelegate {
        var onActivate: (@MainActor () -> Void)?

        let authorizer = NotificationAuthorizer()
        private let center = UNUserNotificationCenter.current()

        func start() {
            center.delegate = self
        }

        func post(_ alert: DaemonAlert) {
            Task {
                guard await authorizer.requestIfNeeded() else { return }
                await Self.deliver(alert)
            }
        }

        private nonisolated static func deliver(_ alert: DaemonAlert) async {
            let content = UNMutableNotificationContent()
            content.title = alert.title
            content.body = alert.body
            if alert.urgency == .critical { content.sound = .default }
            let request = UNNotificationRequest(identifier: alert.id, content: content, trigger: nil)
            try? await UNUserNotificationCenter.current().add(request)
        }

        nonisolated func userNotificationCenter(
            _ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse,
            withCompletionHandler completionHandler: @escaping () -> Void
        ) {
            completionHandler()
            Task { @MainActor [weak self] in self?.onActivate?() }
        }

        nonisolated func userNotificationCenter(
            _ center: UNUserNotificationCenter, willPresent notification: UNNotification,
            withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
        ) {
            completionHandler([.banner, .sound])
        }
    }
#endif
