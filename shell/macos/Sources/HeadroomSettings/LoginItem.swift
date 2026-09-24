#if canImport(AppKit)
    import Foundation
    import Observation
    import ServiceManagement

    @MainActor
    @Observable
    public final class LoginItem {
        public private(set) var status: SMAppService.Status = .notRegistered
        public private(set) var failure: String?

        public init() {}

        public var isEnabled: Bool { status == .enabled }
        public var needsApproval: Bool { status == .requiresApproval }

        public func refresh() {
            status = SMAppService.mainApp.status
        }

        public func setEnabled(_ enabled: Bool) {
            do {
                if enabled {
                    try SMAppService.mainApp.register()
                } else {
                    try SMAppService.mainApp.unregister()
                }
                failure = nil
            } catch {
                failure = error.localizedDescription
            }
            refresh()
        }

        public func openSystemSettings() {
            SMAppService.openSystemSettingsLoginItems()
        }
    }
#endif
