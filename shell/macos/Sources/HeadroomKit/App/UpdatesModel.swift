import Foundation
import Observation

public struct UpdaterStatus: Sendable, Hashable {
    public var automaticallyChecks: Bool
    public var canCheck: Bool
    public var lastCheck: Date?

    public init(automaticallyChecks: Bool, canCheck: Bool, lastCheck: Date?) {
        self.automaticallyChecks = automaticallyChecks
        self.canCheck = canCheck
        self.lastCheck = lastCheck
    }

    public static let unavailable = UpdaterStatus(automaticallyChecks: false, canCheck: false, lastCheck: nil)
}

@MainActor
public protocol UpdateControls: AnyObject {
    var status: UpdaterStatus { get }
    func setAutomaticallyChecks(_ enabled: Bool)
    func checkNow()
}

@MainActor
@Observable
public final class UpdatesModel {
    public private(set) var status: UpdaterStatus = .unavailable
    @ObservationIgnored private weak var controls: (any UpdateControls)?

    public init() {}

    public func connect(_ controls: any UpdateControls) {
        self.controls = controls
        refresh()
    }

    public func refresh() {
        guard let controls, controls.status != status else { return }
        status = controls.status
    }

    public func setAutomaticallyChecks(_ enabled: Bool) {
        controls?.setAutomaticallyChecks(enabled)
        refresh()
    }

    public func checkNow() {
        guard status.canCheck else { return }
        controls?.checkNow()
        refresh()
    }
}
