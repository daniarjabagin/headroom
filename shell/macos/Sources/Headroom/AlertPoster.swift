#if canImport(AppKit)
    import Foundation
    import HeadroomKit
    import UserNotifications

    @MainActor
    final class AlertPoster: NSObject, UNUserNotificationCenterDelegate {
        var onActivate: (@MainActor () -> Void)?

        private let center = UNUserNotificationCenter.current()

        func start() {
            center.delegate = self
            center.requestAuthorization(options: [.alert, .sound]) { _, _ in }
        }

        func post(_ alert: DaemonAlert) {
            let content = UNMutableNotificationContent()
            content.title = alert.title
            content.body = alert.body
            if alert.urgency == .critical { content.sound = .default }
            center.add(UNNotificationRequest(identifier: alert.id, content: content, trigger: nil))
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
